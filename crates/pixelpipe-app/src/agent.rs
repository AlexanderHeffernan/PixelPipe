use std::{
    collections::BTreeMap,
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use pixelpipe_core::{
    apply_palette_remap, apply_pixel_patch, decode_rgba_png, sha256_hex, stable_json,
};
use pixelpipe_project::{
    AGENT_RUN_SCHEMA, AgentCandidate, AgentCapability, AgentIdentity, AgentOperation,
    AgentProposal, AgentRunRecord, AgentRunStatus, ProjectStore, ReferenceSelection,
};
use serde::{Deserialize, Serialize};

use crate::AppError;

const PROFILE_SCHEMA: &str = "pixelpipe.agent-profile/v1";
const REQUEST_SCHEMA: &str = "pixelpipe.agent-request/v1";
const RESPONSE_SCHEMA: &str = "pixelpipe.agent-response/v1";
const EVENT_SCHEMA: &str = "pixelpipe.agent-task-event/v1";
const MAX_CAPTURE_BYTES: usize = 1_048_576;
static TASK_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTaskRequest {
    pub start: PathBuf,
    pub asset: String,
    pub profile: String,
    pub operation: AgentOperation,
    #[serde(default)]
    pub revision: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentTaskResult {
    pub run: AgentRunRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentTaskEvent {
    pub schema: String,
    pub task: String,
    pub sequence: u64,
    pub event: AgentTaskEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTaskEventKind {
    Started {
        operation: AgentOperation,
    },
    Progress {
        stage: String,
        message: String,
    },
    Log {
        stream: String,
        message: String,
    },
    CandidateReady {
        candidate: AgentCandidate,
    },
    Completed {
        run: AgentRunRecord,
    },
    Failed {
        #[serde(skip_serializing_if = "Option::is_none")]
        run: Option<AgentRunRecord>,
        error: String,
    },
    Cancelled {
        run: AgentRunRecord,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentProfile {
    schema: String,
    id: String,
    approved: bool,
    executable: PathBuf,
    #[serde(default)]
    args: Vec<String>,
    capabilities: Vec<AgentCapability>,
    #[serde(default)]
    environment: Vec<String>,
    #[serde(default)]
    secret_environment: Vec<String>,
    #[serde(default = "default_timeout_seconds")]
    timeout_seconds: u64,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ProtocolRequest {
    schema: String,
    id: String,
    operation: AgentOperation,
    asset: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<String>,
    prompt: String,
    workspace: ProtocolWorkspace,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ProtocolWorkspace {
    output_directory: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_png: Option<ProtocolInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_png: Option<ProtocolInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pixels_json: Option<ProtocolInput>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ProtocolInput {
    path: PathBuf,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtocolResponse {
    schema: String,
    adapter: AgentIdentity,
    result: ProtocolResult,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ProtocolResult {
    GeneratedReferences { candidates: Vec<ProtocolCandidate> },
    Critique { text: String },
    Proposal { proposal: AgentProposal },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtocolCandidate {
    id: String,
    path: PathBuf,
    sha256: String,
}

#[derive(Debug)]
struct ProcessCapture {
    exit_status: Option<i32>,
    duration_ms: u64,
    stdout: Vec<u8>,
    stderr: String,
    cancelled: bool,
    timed_out: bool,
    overflowed: bool,
    process_error: Option<String>,
}

struct RunPayload {
    adapter: Option<AgentIdentity>,
    candidates: Vec<AgentCandidate>,
    candidate_bytes: BTreeMap<String, Vec<u8>>,
    critique: Option<String>,
    proposal: Option<AgentProposal>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoadAgentCandidate {
    pub start: PathBuf,
    pub run: String,
    pub candidate: String,
}

#[derive(Debug, Deserialize)]
pub struct BrowseAgentRuns {
    pub start: PathBuf,
    pub asset: String,
}

#[derive(Debug, Deserialize)]
pub struct SelectAgentCandidate {
    pub start: PathBuf,
    pub asset: String,
    pub run: String,
    pub candidate: String,
}

#[derive(Debug, Clone)]
pub struct AgentRuntime {
    profile_directory: PathBuf,
}

impl AgentRuntime {
    /// Uses the operating system's user-local `PixelPipe` agent profile directory.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if no user configuration directory is available.
    pub fn user_local() -> Result<Self, AppError> {
        Ok(Self {
            profile_directory: user_profile_directory()?,
        })
    }

    /// Creates a runtime rooted at an explicit user profile directory.
    /// Intended for embedding and deterministic tests; project manifests never set it.
    #[must_use]
    pub fn with_profile_directory(profile_directory: PathBuf) -> Self {
        Self { profile_directory }
    }

    /// Runs one approved local adapter while emitting a typed task lifecycle.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] for profile, project, process, protocol, candidate,
    /// or immutable run-storage failures.
    pub fn run(
        &self,
        request: AgentTaskRequest,
        cancel: &AtomicBool,
        sink: Arc<dyn Fn(AgentTaskEvent) + Send + Sync>,
    ) -> Result<AgentTaskResult, AppError> {
        self.run_with_task(request, new_task_id()?, cancel, sink)
    }

    /// Allocates a task ID before asynchronous execution begins.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if the system clock cannot produce an ID.
    pub fn allocate_task_id() -> Result<String, AppError> {
        new_task_id()
    }

    /// Runs one approved adapter under a previously allocated task ID.
    ///
    /// # Errors
    ///
    /// Returns the same failures as [`Self::run`].
    pub fn run_with_task(
        &self,
        request: AgentTaskRequest,
        task: String,
        cancel: &AtomicBool,
        sink: Arc<dyn Fn(AgentTaskEvent) + Send + Sync>,
    ) -> Result<AgentTaskResult, AppError> {
        validate_local_id(&task)?;
        let store = ProjectStore::discover(&request.start)?;
        let profile = self.load_profile(&request.profile)?;
        require_capability(&profile, request.operation)?;
        let emitter = EventEmitter::new(task.clone(), sink);
        emitter.emit(AgentTaskEventKind::Started {
            operation: request.operation,
        });

        let started_unix_ms = now_unix_ms()?;
        let task_directory = tempfile::Builder::new()
            .prefix("pixelpipe-agent-")
            .tempdir()
            .map_err(|source| {
                AppError::AgentProcess(format!("cannot create isolated task workspace: {source}"))
            })?;
        let workspace = task_directory.path().to_path_buf();
        let output_directory = workspace.join("output");
        fs::create_dir_all(&output_directory).map_err(|source| {
            AppError::AgentProcess(format!("cannot create isolated task workspace: {source}"))
        })?;
        let protocol = prepare_request(&store, &request, &task, &workspace, &output_directory)?;
        let protocol_bytes = stable_json(&protocol)?;
        let command_hash = sha256_hex(&stable_json(&(
            profile.executable.clone(),
            profile.args.clone(),
        ))?);
        let (environment, secrets) = profile_environment(&profile)?;
        let redacted_prompt = redact(&request.prompt, &secrets);
        emitter.emit(AgentTaskEventKind::Progress {
            stage: "starting_adapter".to_owned(),
            message: format!("Starting approved profile '{}'.", profile.id),
        });

        let capture = execute(
            &profile,
            &workspace,
            &protocol_bytes,
            &environment,
            &secrets,
            cancel,
            &emitter,
        )
        .unwrap_or_else(ProcessCapture::failed);
        let stdout_text = redact(&String::from_utf8_lossy(&capture.stdout), &secrets);
        let payload = interpret_capture(
            &capture,
            &request,
            &profile,
            &store,
            &output_directory,
            &secrets,
            &emitter,
        );
        let status = if capture.cancelled {
            AgentRunStatus::Cancelled
        } else if payload.error.is_some() {
            AgentRunStatus::Failed
        } else {
            AgentRunStatus::Completed
        };
        let record = AgentRunRecord {
            schema: AGENT_RUN_SCHEMA.to_owned(),
            id: task,
            asset: request.asset,
            operation: request.operation,
            revision: request.revision,
            profile: profile.id,
            profile_command_sha256: command_hash,
            prompt: redacted_prompt,
            started_unix_ms,
            duration_ms: capture.duration_ms,
            status,
            exit_status: capture.exit_status,
            adapter: payload.adapter,
            stdout: stdout_text,
            stderr: capture.stderr,
            error: payload.error,
            candidates: payload.candidates,
            critique: payload.critique,
            proposal: payload.proposal,
        };
        store.store_agent_run(&record, &payload.candidate_bytes)?;
        match record.status {
            AgentRunStatus::Completed => emitter.emit(AgentTaskEventKind::Completed {
                run: record.clone(),
            }),
            AgentRunStatus::Failed => emitter.emit(AgentTaskEventKind::Failed {
                error: record
                    .error
                    .clone()
                    .unwrap_or_else(|| "agent task failed".to_owned()),
                run: Some(record.clone()),
            }),
            AgentRunStatus::Cancelled => emitter.emit(AgentTaskEventKind::Cancelled {
                run: record.clone(),
            }),
        }
        Ok(AgentTaskResult { run: record })
    }

    fn load_profile(&self, id: &str) -> Result<AgentProfile, AppError> {
        validate_local_id(id)?;
        let path = self.profile_directory.join(format!("{id}.json"));
        let bytes = fs::read(&path).map_err(|source| {
            AppError::AgentProfile(format!("cannot read profile '{id}': {source}"))
        })?;
        let profile: AgentProfile = serde_json::from_slice(&bytes).map_err(|source| {
            AppError::AgentProfile(format!("invalid profile '{id}': {source}"))
        })?;
        if profile.schema != PROFILE_SCHEMA || profile.id != id {
            return Err(AppError::AgentProfile(format!(
                "profile '{id}' has an unsupported schema or mismatched identity"
            )));
        }
        if !profile.approved {
            return Err(AppError::AgentProfile(format!(
                "profile '{id}' is not explicitly approved"
            )));
        }
        if !profile.executable.is_absolute() {
            return Err(AppError::AgentProfile(format!(
                "profile '{id}' executable must be an absolute path"
            )));
        }
        if profile.timeout_seconds == 0 || profile.timeout_seconds > 86_400 {
            return Err(AppError::AgentProfile(
                "timeout_seconds must be between 1 and 86400".to_owned(),
            ));
        }
        Ok(profile)
    }
}

impl ProcessCapture {
    fn failed(message: String) -> Self {
        Self {
            exit_status: None,
            duration_ms: 0,
            stdout: Vec::new(),
            stderr: String::new(),
            cancelled: false,
            timed_out: false,
            overflowed: false,
            process_error: Some(message),
        }
    }
}

struct ProcessedResponse {
    adapter: AgentIdentity,
    candidates: Vec<AgentCandidate>,
    candidate_bytes: BTreeMap<String, Vec<u8>>,
    critique: Option<String>,
    proposal: Option<AgentProposal>,
}

struct ResponseContext<'a> {
    operation: AgentOperation,
    asset: &'a str,
    revision: Option<&'a str>,
    store: &'a ProjectStore,
    output_directory: &'a Path,
    secrets: &'a [String],
    emitter: &'a EventEmitter,
}

struct EventEmitter {
    task: String,
    sequence: Arc<AtomicU64>,
    sink: Arc<dyn Fn(AgentTaskEvent) + Send + Sync>,
}

impl EventEmitter {
    fn new(task: String, sink: Arc<dyn Fn(AgentTaskEvent) + Send + Sync>) -> Self {
        Self {
            task,
            sequence: Arc::new(AtomicU64::new(1)),
            sink,
        }
    }

    fn emit(&self, event: AgentTaskEventKind) {
        (self.sink)(AgentTaskEvent {
            schema: EVENT_SCHEMA.to_owned(),
            task: self.task.clone(),
            sequence: self.sequence.fetch_add(1, Ordering::Relaxed),
            event,
        });
    }
}

fn prepare_request(
    store: &ProjectStore,
    request: &AgentTaskRequest,
    task: &str,
    workspace: &Path,
    output_directory: &Path,
) -> Result<ProtocolRequest, AppError> {
    let (native_png, preview_png, pixels_json) = match request.operation {
        AgentOperation::GenerateReferences => {
            if request.revision.is_some() {
                return Err(AppError::AgentProtocol(
                    "generation must not name a revision".to_owned(),
                ));
            }
            (None, None, None)
        }
        AgentOperation::CritiqueAsset | AgentOperation::ProposeRefinement => {
            let revision = request.revision.as_deref().ok_or_else(|| {
                AppError::AgentProtocol(
                    "critique/proposal requires an explicit revision".to_owned(),
                )
            })?;
            let snapshot = store.revision(&request.asset, revision)?;
            let input = workspace.join("input");
            fs::create_dir_all(&input).map_err(|source| {
                AppError::AgentProcess(format!("cannot create task input directory: {source}"))
            })?;
            let native = input.join("native.png");
            let preview = input.join("preview.png");
            let pixels = input.join("pixels.json");
            let pixel_bytes = stable_json(&snapshot.raster)?;
            write_task_file(&native, &snapshot.native_png)?;
            write_task_file(&preview, &snapshot.preview_png)?;
            write_task_file(&pixels, &pixel_bytes)?;
            (
                Some(protocol_input(native, &snapshot.native_png)),
                Some(protocol_input(preview, &snapshot.preview_png)),
                Some(protocol_input(pixels, &pixel_bytes)),
            )
        }
    };
    Ok(ProtocolRequest {
        schema: REQUEST_SCHEMA.to_owned(),
        id: task.to_owned(),
        operation: request.operation,
        asset: request.asset.clone(),
        revision: request.revision.clone(),
        prompt: request.prompt.clone(),
        workspace: ProtocolWorkspace {
            output_directory: output_directory.to_path_buf(),
            native_png,
            preview_png,
            pixels_json,
        },
    })
}

fn protocol_input(path: PathBuf, bytes: &[u8]) -> ProtocolInput {
    ProtocolInput {
        path,
        sha256: sha256_hex(bytes),
    }
}

fn execute(
    profile: &AgentProfile,
    workspace: &Path,
    request: &[u8],
    environment: &BTreeMap<String, String>,
    secrets: &[String],
    cancel: &AtomicBool,
    emitter: &EventEmitter,
) -> Result<ProcessCapture, String> {
    let started = Instant::now();
    let mut command = Command::new(&profile.executable);
    command
        .args(&profile.args)
        .current_dir(workspace)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|source| format!("cannot start approved profile '{}': {source}", profile.id))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "adapter stdin is unavailable".to_owned())?
        .write_all(request)
        .map_err(|source| format!("cannot write adapter request: {source}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "adapter stdout is unavailable".to_owned())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "adapter stderr is unavailable".to_owned())?;
    let stdout_reader = thread::spawn(move || read_capped(stdout));
    let secret_values = secrets.to_vec();
    let sink = Arc::clone(&emitter.sink);
    let task = emitter.task.clone();
    let sequence = Arc::clone(&emitter.sequence);
    let stderr_reader =
        thread::spawn(move || read_stderr(stderr, &secret_values, &sink, &task, &sequence));

    let mut cancelled = false;
    let mut timed_out = false;
    let exit_status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {}
            Err(source) => return Err(format!("cannot wait for adapter: {source}")),
        }
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            child
                .kill()
                .map_err(|source| format!("cannot cancel adapter: {source}"))?;
            break child.wait().ok().and_then(|status| status.code());
        }
        if started.elapsed() >= Duration::from_secs(profile.timeout_seconds) {
            timed_out = true;
            child
                .kill()
                .map_err(|source| format!("cannot stop timed-out adapter: {source}"))?;
            break child.wait().ok().and_then(|status| status.code());
        }
        thread::sleep(Duration::from_millis(20));
    };
    let (stdout, stdout_overflow) = stdout_reader
        .join()
        .map_err(|_| "adapter stdout reader panicked".to_owned())?;
    let (stderr, stderr_overflow) = stderr_reader
        .join()
        .map_err(|_| "adapter stderr reader panicked".to_owned())?;
    Ok(ProcessCapture {
        exit_status,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        stdout,
        stderr,
        cancelled,
        timed_out,
        overflowed: stdout_overflow || stderr_overflow,
        process_error: None,
    })
}

fn interpret_capture(
    capture: &ProcessCapture,
    request: &AgentTaskRequest,
    profile: &AgentProfile,
    store: &ProjectStore,
    output_directory: &Path,
    secrets: &[String],
    emitter: &EventEmitter,
) -> RunPayload {
    let error = capture.process_error.clone().or_else(|| {
        if capture.cancelled {
            Some("task cancelled by user".to_owned())
        } else if capture.timed_out {
            Some(format!(
                "adapter exceeded {} second timeout",
                profile.timeout_seconds
            ))
        } else if capture.overflowed {
            Some("adapter output exceeded the 1 MiB capture limit".to_owned())
        } else if capture.exit_status != Some(0) {
            Some(format!(
                "adapter exited with status {}",
                capture
                    .exit_status
                    .map_or_else(|| "unknown".to_owned(), |code| code.to_string())
            ))
        } else {
            None
        }
    });
    if let Some(error) = error {
        return failed_payload(redact(&error, secrets));
    }
    let context = ResponseContext {
        operation: request.operation,
        asset: &request.asset,
        revision: request.revision.as_deref(),
        store,
        output_directory,
        secrets,
        emitter,
    };
    match process_response(&capture.stdout, &context) {
        Ok(result) => RunPayload {
            adapter: Some(result.adapter),
            candidates: result.candidates,
            candidate_bytes: result.candidate_bytes,
            critique: result.critique,
            proposal: result.proposal,
            error: None,
        },
        Err(error) => failed_payload(redact(&error.to_string(), secrets)),
    }
}

fn failed_payload(error: String) -> RunPayload {
    RunPayload {
        adapter: None,
        candidates: Vec::new(),
        candidate_bytes: BTreeMap::new(),
        critique: None,
        proposal: None,
        error: Some(error),
    }
}

fn process_response(
    bytes: &[u8],
    context: &ResponseContext<'_>,
) -> Result<ProcessedResponse, AppError> {
    let mut response: ProtocolResponse = serde_json::from_slice(bytes)
        .map_err(|source| AppError::AgentProtocol(format!("invalid JSON response: {source}")))?;
    if response.schema != RESPONSE_SCHEMA {
        return Err(AppError::AgentProtocol(format!(
            "unsupported response schema '{}'",
            response.schema
        )));
    }
    if !response
        .adapter
        .capabilities
        .contains(&required_capability(context.operation))
    {
        return Err(AppError::AgentProtocol(
            "adapter did not report the requested capability".to_owned(),
        ));
    }
    response.adapter.adapter = redact(&response.adapter.adapter, context.secrets);
    response.adapter.provider = response
        .adapter
        .provider
        .map(|value| redact(&value, context.secrets));
    response.adapter.model = response
        .adapter
        .model
        .map(|value| redact(&value, context.secrets));
    let mut processed = ProcessedResponse {
        adapter: response.adapter,
        candidates: Vec::new(),
        candidate_bytes: BTreeMap::new(),
        critique: None,
        proposal: None,
    };
    match (context.operation, response.result) {
        (
            AgentOperation::GenerateReferences,
            ProtocolResult::GeneratedReferences { candidates },
        ) => {
            if candidates.is_empty() {
                return Err(AppError::AgentProtocol(
                    "generation returned no candidates".to_owned(),
                ));
            }
            for candidate in candidates {
                let (metadata, bytes) = validate_candidate(context.output_directory, candidate)?;
                context.emitter.emit(AgentTaskEventKind::CandidateReady {
                    candidate: metadata.clone(),
                });
                processed.candidate_bytes.insert(metadata.id.clone(), bytes);
                processed.candidates.push(metadata);
            }
        }
        (AgentOperation::CritiqueAsset, ProtocolResult::Critique { text }) => {
            if text.trim().is_empty() {
                return Err(AppError::AgentProtocol("critique is empty".to_owned()));
            }
            processed.critique = Some(redact(&text, context.secrets));
        }
        (AgentOperation::ProposeRefinement, ProtocolResult::Proposal { proposal }) => {
            let revision = context.revision.ok_or_else(|| {
                AppError::AgentProtocol("proposal requires a revision".to_owned())
            })?;
            // Validation is intentionally read-only. Applying still requires the
            // existing explicit-parent revision use case.
            let snapshot = context.store.revision(context.asset, revision)?;
            match &proposal {
                AgentProposal::PixelPatch { patch } => {
                    let mut patch = patch.clone();
                    crate::inherit_structure(
                        &mut patch.structure,
                        crate::component_rule(&snapshot.recipe),
                    )?;
                    apply_pixel_patch(&snapshot.raster, &patch)?;
                }
                AgentProposal::PaletteRemap { remap } => {
                    let mut remap = remap.clone();
                    crate::inherit_structure(
                        &mut remap.structure,
                        crate::component_rule(&snapshot.recipe),
                    )?;
                    apply_palette_remap(&snapshot.raster, &remap)?;
                }
            }
            processed.proposal = Some(proposal);
        }
        _ => {
            return Err(AppError::AgentProtocol(
                "response result does not match requested operation".to_owned(),
            ));
        }
    }
    Ok(processed)
}

fn validate_candidate(
    output_directory: &Path,
    candidate: ProtocolCandidate,
) -> Result<(AgentCandidate, Vec<u8>), AppError> {
    validate_local_id(&candidate.id)?;
    if candidate.path.is_absolute() {
        return Err(AppError::AgentCandidatePath(
            "absolute paths are not accepted".to_owned(),
        ));
    }
    let root = output_directory.canonicalize().map_err(|source| {
        AppError::AgentCandidatePath(format!("cannot verify output directory: {source}"))
    })?;
    let path = output_directory.join(&candidate.path);
    let canonical = path.canonicalize().map_err(|source| {
        AppError::AgentCandidatePath(format!("cannot verify '{}': {source}", candidate.id))
    })?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return Err(AppError::AgentCandidatePath(format!(
            "candidate '{}' escapes its assigned output directory",
            candidate.id
        )));
    }
    let bytes = fs::read(&canonical).map_err(|source| {
        AppError::AgentCandidatePath(format!("cannot read '{}': {source}", candidate.id))
    })?;
    let actual_hash = sha256_hex(&bytes);
    if actual_hash != candidate.sha256 {
        return Err(AppError::AgentProtocol(format!(
            "candidate '{}' hash does not match its bytes",
            candidate.id
        )));
    }
    let image = decode_rgba_png(&bytes)?;
    Ok((
        AgentCandidate {
            id: candidate.id,
            sha256: actual_hash.clone(),
            width: image.width,
            height: image.height,
            png: format!("candidates/{actual_hash}.png"),
        },
        bytes,
    ))
}

/// Loads verified candidate bytes for a short UI/CLI request.
///
/// # Errors
///
/// Returns an [`AppError`] if the run, candidate, or hash is invalid.
pub fn load_agent_candidate(request: LoadAgentCandidate) -> Result<Vec<u8>, AppError> {
    let LoadAgentCandidate {
        start,
        run,
        candidate,
    } = request;
    let store = ProjectStore::discover(&start)?;
    let run_record = store.agent_run(&run)?;
    let candidate = run_record
        .candidates
        .iter()
        .find(|entry| entry.id == candidate)
        .ok_or_else(|| AppError::AgentProtocol("candidate does not exist in run".to_owned()))?;
    let path = store
        .root()
        .join(".pixelpipe/runs")
        .join(run)
        .join(&candidate.png);
    let bytes = fs::read(&path).map_err(|source| AppError::Read { path, source })?;
    if sha256_hex(&bytes) != candidate.sha256 {
        return Err(AppError::AgentProtocol(
            "stored candidate hash mismatch".to_owned(),
        ));
    }
    Ok(bytes)
}

/// Lists hash-verified local agent runs for one asset.
///
/// # Errors
///
/// Returns an [`AppError`] if project discovery or run verification fails.
pub fn browse_agent_runs(request: BrowseAgentRuns) -> Result<Vec<AgentRunRecord>, AppError> {
    let BrowseAgentRuns { start, asset } = request;
    let store = ProjectStore::discover(&start)?;
    store.agent_runs(&asset).map_err(AppError::from)
}

/// Selects a validated candidate without changing asset head or approval.
///
/// # Errors
///
/// Returns an [`AppError`] when the project/run/candidate cannot be verified.
pub fn select_agent_candidate(
    request: SelectAgentCandidate,
) -> Result<ReferenceSelection, AppError> {
    let SelectAgentCandidate {
        start,
        asset,
        run,
        candidate,
    } = request;
    let store = ProjectStore::discover(&start)?;
    store
        .select_agent_candidate(&asset, &run, &candidate)
        .map_err(AppError::from)
}

fn require_capability(profile: &AgentProfile, operation: AgentOperation) -> Result<(), AppError> {
    let capability = required_capability(operation);
    if profile.capabilities.contains(&capability) {
        Ok(())
    } else {
        Err(AppError::AgentProfile(format!(
            "profile '{}' is not allowlisted for {operation:?}",
            profile.id
        )))
    }
}

const fn required_capability(operation: AgentOperation) -> AgentCapability {
    match operation {
        AgentOperation::GenerateReferences => AgentCapability::GenerateReferences,
        AgentOperation::CritiqueAsset => AgentCapability::CritiqueAsset,
        AgentOperation::ProposeRefinement => AgentCapability::ProposeRefinement,
    }
}

fn profile_environment(
    profile: &AgentProfile,
) -> Result<(BTreeMap<String, String>, Vec<String>), AppError> {
    let mut names = profile.environment.clone();
    names.extend(profile.secret_environment.iter().cloned());
    names.sort();
    names.dedup();
    let mut values = BTreeMap::new();
    for name in names {
        let value = env::var(&name).map_err(|_| {
            AppError::AgentProfile(format!("required environment variable '{name}' is not set"))
        })?;
        values.insert(name, value);
    }
    let secrets = profile
        .secret_environment
        .iter()
        .filter_map(|name| values.get(name).cloned())
        .filter(|value| !value.is_empty())
        .collect();
    Ok((values, secrets))
}

fn redact(value: &str, secrets: &[String]) -> String {
    secrets.iter().fold(value.to_owned(), |redacted, secret| {
        redacted.replace(secret, "[REDACTED]")
    })
}

fn read_capped(mut reader: impl Read) -> (Vec<u8>, bool) {
    let mut captured = Vec::new();
    let mut overflowed = false;
    let mut buffer = [0_u8; 8192];
    while let Ok(read) = reader.read(&mut buffer) {
        if read == 0 {
            break;
        }
        let remaining = MAX_CAPTURE_BYTES.saturating_sub(captured.len());
        captured.extend_from_slice(&buffer[..read.min(remaining)]);
        overflowed |= read > remaining;
    }
    (captured, overflowed)
}

fn read_stderr(
    reader: impl Read,
    secrets: &[String],
    sink: &Arc<dyn Fn(AgentTaskEvent) + Send + Sync>,
    task: &str,
    sequence: &Arc<AtomicU64>,
) -> (String, bool) {
    let (bytes, overflowed) = read_capped(reader);
    let captured = redact(&String::from_utf8_lossy(&bytes), secrets);
    for line in captured.lines() {
        sink(AgentTaskEvent {
            schema: EVENT_SCHEMA.to_owned(),
            task: task.to_owned(),
            sequence: sequence.fetch_add(1, Ordering::Relaxed),
            event: AgentTaskEventKind::Log {
                stream: "stderr".to_owned(),
                message: line.to_owned(),
            },
        });
    }
    (captured, overflowed)
}

fn user_profile_directory() -> Result<PathBuf, AppError> {
    #[cfg(target_os = "windows")]
    let root = env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let root = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let root = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".config"))
        });
    root.map(|root| root.join("pixelpipe/agents"))
        .ok_or_else(|| {
            AppError::AgentProfile("user configuration directory is unavailable".to_owned())
        })
}

fn validate_local_id(id: &str) -> Result<(), AppError> {
    let valid = !id.is_empty()
        && id.len() <= 96
        && !id.starts_with('-')
        && !id.ends_with('-')
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(AppError::AgentProfile(format!(
            "invalid local identifier '{id}'"
        )))
    }
}

fn write_task_file(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    fs::write(path, bytes).map_err(|source| {
        AppError::AgentProcess(format!(
            "cannot prepare task input '{}': {source}",
            path.display()
        ))
    })
}

fn default_timeout_seconds() -> u64 {
    300
}

fn now_unix_ms() -> Result<u64, AppError> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AppError::AgentProcess("system clock is before Unix epoch".to_owned()))?
            .as_millis(),
    )
    .map_err(|_| AppError::AgentProcess("system clock overflow".to_owned()))
}

fn new_task_id() -> Result<String, AppError> {
    let now = now_unix_ms()?;
    let sequence = TASK_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(format!("t{now}-{}-{sequence}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use pixelpipe_project::{AgentRunStatus, AssetKind};
    use tempfile::{TempDir, tempdir};

    use super::*;
    use crate::{CreateRevision, create_revision};

    struct FixtureProject {
        root: TempDir,
        profiles: TempDir,
        revision: String,
    }

    impl FixtureProject {
        fn new() -> Self {
            let root = tempdir().expect("project root");
            ProjectStore::init(root.path(), "Agent Fixture").expect("project init");
            let raster = fixture_path("../../fixtures/m1/tiny-raster.json");
            let revision = create_revision(CreateRevision {
                start: root.path().to_path_buf(),
                asset: "signal-flare".to_owned(),
                kind: AssetKind::Sprite,
                raster_path: raster,
                brief_path: None,
                preview_scale: Some(4),
                actor: "fixture".to_owned(),
            })
            .expect("fixture revision")
            .revision;
            Self {
                root,
                profiles: tempdir().expect("profiles"),
                revision,
            }
        }

        fn profile(&self, id: &str, mode: &str, approved: bool) {
            let profile = serde_json::json!({
                "schema": PROFILE_SCHEMA,
                "id": id,
                "approved": approved,
                "executable": "/usr/bin/python3",
                "args": [fixture_path("../../fixtures/m5/fake_agent.py"), mode],
                "capabilities": [
                    "generate_references",
                    "critique_asset",
                    "propose_refinement"
                ],
                "secret_environment": ["HOME"],
                "timeout_seconds": 2
            });
            fs::write(
                self.profiles.path().join(format!("{id}.json")),
                serde_json::to_vec_pretty(&profile).expect("profile JSON"),
            )
            .expect("write profile");
        }

        fn request(&self, profile: &str, operation: AgentOperation) -> AgentTaskRequest {
            AgentTaskRequest {
                start: self.root.path().to_path_buf(),
                asset: "signal-flare".to_owned(),
                profile: profile.to_owned(),
                operation,
                revision: (operation != AgentOperation::GenerateReferences)
                    .then(|| self.revision.clone()),
                prompt: format!(
                    "Synthetic fixture prompt {}",
                    env::var("HOME").expect("HOME")
                ),
            }
        }

        fn run(
            &self,
            request: AgentTaskRequest,
        ) -> (AgentTaskResult, Arc<Mutex<Vec<AgentTaskEvent>>>) {
            let events = Arc::new(Mutex::new(Vec::new()));
            let captured = Arc::clone(&events);
            let sink = Arc::new(move |event| captured.lock().expect("events").push(event));
            let cancel = AtomicBool::new(false);
            let result = AgentRuntime::with_profile_directory(self.profiles.path().to_path_buf())
                .run(request, &cancel, sink)
                .expect("run result");
            (result, events)
        }
    }

    #[test]
    fn generation_captures_redacted_provenance_and_requires_explicit_selection() {
        let fixture = FixtureProject::new();
        fixture.profile("fixture-success", "success", true);
        let store = ProjectStore::discover(fixture.root.path()).expect("store");
        let before = store.asset("signal-flare").expect("asset").head;

        let (result, events) =
            fixture.run(fixture.request("fixture-success", AgentOperation::GenerateReferences));

        assert_eq!(result.run.status, AgentRunStatus::Completed);
        assert_eq!(result.run.candidates.len(), 1);
        assert_eq!(
            result.run.candidates[0].sha256,
            "be0cb8da780b0a4a59a38302b855c657bfd997de8e8883a1db42a4698dba3ede"
        );
        assert!(result.run.prompt.contains("[REDACTED]"));
        assert!(result.run.stderr.contains("[REDACTED]"));
        assert!(!result.run.stderr.contains(&env::var("HOME").expect("HOME")));
        assert_eq!(store.asset("signal-flare").expect("asset").head, before);
        let event_guard = events.lock().expect("events");
        let event_types = event_guard
            .iter()
            .map(|event| &event.event)
            .collect::<Vec<_>>();
        assert!(
            event_types
                .iter()
                .any(|event| matches!(event, AgentTaskEventKind::Started { .. }))
        );
        assert!(
            event_types
                .iter()
                .any(|event| matches!(event, AgentTaskEventKind::Log { .. }))
        );
        assert!(
            event_types
                .iter()
                .any(|event| matches!(event, AgentTaskEventKind::CandidateReady { .. }))
        );
        assert!(
            event_types
                .iter()
                .any(|event| matches!(event, AgentTaskEventKind::Completed { .. }))
        );

        let candidate = result.run.candidates[0].clone();
        let bytes = load_agent_candidate(LoadAgentCandidate {
            start: fixture.root.path().to_path_buf(),
            run: result.run.id.clone(),
            candidate: candidate.id.clone(),
        })
        .expect("candidate bytes");
        assert_eq!(sha256_hex(&bytes), candidate.sha256);
        let selection = select_agent_candidate(SelectAgentCandidate {
            start: fixture.root.path().to_path_buf(),
            asset: "signal-flare".to_owned(),
            run: result.run.id,
            candidate: candidate.id.clone(),
        })
        .expect("selection");
        assert_eq!(selection.sha256, candidate.sha256);
        assert_eq!(store.asset("signal-flare").expect("asset").head, before);
    }

    #[test]
    fn critique_and_valid_proposal_are_recorded_without_applying_or_approving() {
        let fixture = FixtureProject::new();
        fixture.profile("fixture-review", "success", true);
        let store = ProjectStore::discover(fixture.root.path()).expect("store");
        let before = store.asset("signal-flare").expect("asset");

        let (critique, _) =
            fixture.run(fixture.request("fixture-review", AgentOperation::CritiqueAsset));
        assert!(
            critique
                .run
                .critique
                .as_deref()
                .is_some_and(|text| { text.contains("Silhouette") && text.contains("[REDACTED]") })
        );
        let (proposal, _) =
            fixture.run(fixture.request("fixture-review", AgentOperation::ProposeRefinement));
        assert!(matches!(
            proposal.run.proposal,
            Some(AgentProposal::PixelPatch { .. })
        ));

        let after = store.asset("signal-flare").expect("asset");
        assert_eq!(after.head, before.head);
        assert_eq!(after.approved, before.approved);
        assert_eq!(store.revisions("signal-flare").expect("revisions").len(), 1);
    }

    #[test]
    fn rejects_unapproved_profiles_without_starting_a_run() {
        let fixture = FixtureProject::new();
        fixture.profile("fixture-unapproved", "success", false);
        let runtime = AgentRuntime::with_profile_directory(fixture.profiles.path().to_path_buf());
        let error = runtime
            .run(
                fixture.request("fixture-unapproved", AgentOperation::GenerateReferences),
                &AtomicBool::new(false),
                Arc::new(|_| {}),
            )
            .expect_err("profile must be refused");
        assert!(error.to_string().contains("not explicitly approved"));
        assert!(
            ProjectStore::discover(fixture.root.path())
                .expect("store")
                .agent_runs("signal-flare")
                .expect("runs")
                .is_empty()
        );
    }

    #[test]
    fn failed_protocols_and_candidates_are_durable_failed_runs() {
        for (id, mode, expected) in [
            ("fixture-malformed", "malformed", "invalid JSON response"),
            ("fixture-exit", "exit-failure", "status 7"),
            ("fixture-hash", "bad-hash", "hash does not match"),
            ("fixture-escape", "escape", "assigned output directory"),
        ] {
            let fixture = FixtureProject::new();
            fixture.profile(id, mode, true);
            let (result, _) = fixture.run(fixture.request(id, AgentOperation::GenerateReferences));
            assert_eq!(result.run.status, AgentRunStatus::Failed, "{mode}");
            assert!(
                result
                    .run
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains(expected)),
                "{mode}: {:?}",
                result.run.error
            );
            assert!(result.run.candidates.is_empty());
            let stored = ProjectStore::discover(fixture.root.path())
                .expect("store")
                .agent_run(&result.run.id)
                .expect("failed run");
            assert_eq!(stored.status, AgentRunStatus::Failed);
        }
    }

    #[test]
    fn cancellation_kills_the_process_and_records_cancelled_lifecycle() {
        let fixture = FixtureProject::new();
        fixture.profile("fixture-cancel", "cancel", true);
        let runtime = AgentRuntime::with_profile_directory(fixture.profiles.path().to_path_buf());
        let request = fixture.request("fixture-cancel", AgentOperation::GenerateReferences);
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let worker = thread::spawn(move || {
            runtime
                .run(request, worker_cancel.as_ref(), Arc::new(|_| {}))
                .expect("cancelled result")
        });
        thread::sleep(Duration::from_millis(100));
        cancel.store(true, Ordering::Relaxed);
        let result = worker.join().expect("worker");

        assert_eq!(result.run.status, AgentRunStatus::Cancelled);
        assert!(result.run.duration_ms < 2_000);
    }

    fn fixture_path(relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
    }
}
