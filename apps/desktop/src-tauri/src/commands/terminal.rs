use std::{
    collections::HashMap,
    env,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use super::CommandResult;

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
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_owned());
    let mut command = CommandBuilder::new(&shell);
    if env::consts::OS == "macos" {
        for argument in macos_shell_arguments(Path::new(&shell)) {
            command.arg(argument);
        }
    }
    command.cwd(&cwd);
    command.env("TERM", "xterm-256color");
    if let Some(path) = terminal_path() {
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
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| error.to_string())?;
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

pub(super) fn macos_shell_arguments(shell: &Path) -> &'static [&'static str] {
    match shell.file_name().and_then(|name| name.to_str()) {
        Some("zsh") => &["-l", "-i"],
        Some("bash") => &["--login", "-i"],
        _ => &[],
    }
}

fn terminal_path() -> Option<String> {
    let mut paths = Vec::new();
    if let Some(directory) = env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(PathBuf::from))
        .filter(|directory| {
            directory
                .join(format!("pixelate{}", env::consts::EXE_SUFFIX))
                .is_file()
        })
    {
        paths.push(directory);
    }
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
