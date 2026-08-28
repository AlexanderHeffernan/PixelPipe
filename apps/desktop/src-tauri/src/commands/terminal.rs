use std::{
    collections::HashMap,
    env,
    io::{Read, Write},
    path::Path,
    sync::Mutex,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use super::{CommandResult, cli_install::bundled_cli_path};

pub(crate) struct TerminalSessions(Mutex<HashMap<String, TerminalSession>>);

struct TerminalSession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

#[derive(Clone, Serialize)]
struct TerminalOutput {
    session: String,
    data: String,
}

impl Default for TerminalSessions {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn start_terminal(
    app: AppHandle,
    sessions: State<'_, TerminalSessions>,
    session: String,
    cwd: String,
    cols: u16,
    rows: u16,
) -> CommandResult<()> {
    let mut sessions = sessions.0.lock().map_err(|error| error.to_string())?;
    if sessions.contains_key(&session) {
        return Ok(());
    }
    let pair = native_pty_system()
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| error.to_string())?;
    let shell =
        std::env::var("SHELL").unwrap_or_else(|_| fallback_shell(env::consts::OS).to_owned());
    let mut command = CommandBuilder::new(&shell);
    let (arguments, path_setup) = shell_configuration(Path::new(&shell), env::consts::OS);
    for argument in arguments {
        command.arg(argument);
    }
    command.cwd(&cwd);
    command.env("TERM", "xterm-256color");
    let cli_path = bundled_cli_path();
    let cli_directory = cli_path.as_deref().and_then(Path::parent);
    if let Some(directory) = cli_directory {
        command.env("PIXELATE_CLI_DIR", directory);
    }
    if let Some(path) = terminal_path(cli_directory) {
        command.env("PATH", path);
    }
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| error.to_string())?;
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| error.to_string())?;
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|error| error.to_string())?;
    if cli_directory.is_some()
        && let Some(setup) = path_setup
    {
        writer
            .write_all(setup.as_bytes())
            .and_then(|()| writer.flush())
            .map_err(|error| error.to_string())?;
    }
    let event_session = session.clone();
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 {
                break;
            }
            let _ = app.emit(
                "terminal-output",
                TerminalOutput {
                    session: event_session.clone(),
                    data: STANDARD.encode(&buffer[..count]),
                },
            );
        }
    });
    sessions.insert(
        session,
        TerminalSession {
            master: pair.master,
            writer,
            child,
        },
    );
    Ok(())
}

pub(super) fn shell_configuration(
    shell: &Path,
    os: &str,
) -> (&'static [&'static str], Option<&'static str>) {
    const RESTORE_POSIX_CLI_PATH: &str =
        "export PATH=\"$PIXELATE_CLI_DIR:$PATH\"; printf '\\033[1A\\033[2K\\r'\n";
    const RESTORE_FISH_CLI_PATH: &str =
        "set -gx PATH \"$PIXELATE_CLI_DIR\" $PATH; printf '\\033[1A\\033[2K\\r'\n";
    match (os, shell.file_name().and_then(|name| name.to_str())) {
        ("macos", Some("zsh")) => (&["-l", "-i"], Some(RESTORE_POSIX_CLI_PATH)),
        ("macos", Some("bash")) => (&["--login", "-i"], Some(RESTORE_POSIX_CLI_PATH)),
        ("linux", Some("bash" | "zsh" | "sh")) => (&[], Some(RESTORE_POSIX_CLI_PATH)),
        ("linux", Some("fish")) => (&[], Some(RESTORE_FISH_CLI_PATH)),
        _ => (&[], None),
    }
}

pub(super) fn fallback_shell(os: &str) -> &'static str {
    if os == "macos" { "/bin/zsh" } else { "/bin/sh" }
}

fn terminal_path(cli_directory: Option<&Path>) -> Option<String> {
    let mut paths = cli_directory
        .into_iter()
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();
    if let Some(path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&path));
    }
    env::join_paths(paths)
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_terminal(
    sessions: State<'_, TerminalSessions>,
    session: String,
    data: String,
) -> CommandResult<()> {
    let mut sessions = sessions.0.lock().map_err(|error| error.to_string())?;
    let terminal = sessions
        .get_mut(&session)
        .ok_or_else(|| "terminal session is not running".to_owned())?;
    terminal
        .writer
        .write_all(data.as_bytes())
        .map_err(|error| error.to_string())?;
    terminal.writer.flush().map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn resize_terminal(
    sessions: State<'_, TerminalSessions>,
    session: String,
    cols: u16,
    rows: u16,
) -> CommandResult<()> {
    let sessions = sessions.0.lock().map_err(|error| error.to_string())?;
    let terminal = sessions
        .get(&session)
        .ok_or_else(|| "terminal session is not running".to_owned())?;
    terminal
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn close_terminal(
    sessions: State<'_, TerminalSessions>,
    session: String,
) -> CommandResult<()> {
    let mut sessions = sessions.0.lock().map_err(|error| error.to_string())?;
    if let Some(mut terminal) = sessions.remove(&session) {
        terminal.child.kill().map_err(|error| error.to_string())?;
    }
    Ok(())
}
