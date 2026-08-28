use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::{CliInstallState, CliInstallStatus, status};
use crate::commands::CommandResult;

const COMMAND_PATH: &str = "/usr/local/bin/pixelate";

pub(super) fn current_status(source: Option<&Path>) -> CliInstallStatus {
    inspect_installation(source, Path::new(COMMAND_PATH))
}

pub(in crate::commands) fn inspect_installation(
    source: Option<&Path>,
    command: &Path,
) -> CliInstallStatus {
    let Some(source) = source else {
        return status(CliInstallState::Unavailable, command.to_path_buf(), false);
    };
    let state = match fs::symlink_metadata(command) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => CliInstallState::NotInstalled,
        Err(_) => CliInstallState::Conflict,
        Ok(metadata) if !metadata.file_type().is_symlink() => CliInstallState::Conflict,
        Ok(_) => match fs::read_link(command) {
            Ok(destination) if destination == source => CliInstallState::Installed,
            Ok(destination) if is_pixelate_link(&destination) => CliInstallState::NeedsRepair,
            _ => CliInstallState::Conflict,
        },
    };
    status(state, command.to_path_buf(), true)
}

pub(super) fn install(source: Option<PathBuf>) -> CommandResult<CliInstallStatus> {
    let source = source.ok_or_else(|| "the bundled Pixelate CLI is unavailable".to_owned())?;
    let command = PathBuf::from(COMMAND_PATH);
    let before = inspect_installation(Some(&source), &command);
    match before.state {
        CliInstallState::Installed => return Ok(before),
        CliInstallState::Conflict => {
            return Err(format!("another command already exists at {COMMAND_PATH}"));
        }
        CliInstallState::Unavailable => return Err("CLI installation is unavailable".to_owned()),
        CliInstallState::NotInstalled | CliInstallState::NeedsRepair => {}
    }
    let expected = (before.state == CliInstallState::NeedsRepair)
        .then(|| fs::read_link(&command))
        .transpose()
        .map_err(|error| error.to_string())?;
    if let Err(error) = replace_link(&source, &command, expected.as_deref()) {
        if error.kind() != io::ErrorKind::PermissionDenied {
            return Err(error.to_string());
        }
        authorize(link_script(&source, &command, expected.as_deref()))?;
    }
    let after = inspect_installation(Some(&source), &command);
    (after.state == CliInstallState::Installed)
        .then_some(after)
        .ok_or_else(|| "Pixelate CLI installation did not complete".to_owned())
}

pub(super) fn uninstall(source: Option<PathBuf>) -> CommandResult<CliInstallStatus> {
    let source = source.ok_or_else(|| "the bundled Pixelate CLI is unavailable".to_owned())?;
    let command = PathBuf::from(COMMAND_PATH);
    let before = inspect_installation(Some(&source), &command);
    if before.state == CliInstallState::NotInstalled {
        return Ok(before);
    }
    if !matches!(
        before.state,
        CliInstallState::Installed | CliInstallState::NeedsRepair
    ) {
        return Err(format!(
            "Pixelate does not manage the command at {COMMAND_PATH}"
        ));
    }
    let expected = fs::read_link(&command).map_err(|error| error.to_string())?;
    if let Err(error) = remove_link(&command, &expected) {
        if error.kind() != io::ErrorKind::PermissionDenied {
            return Err(error.to_string());
        }
        authorize(remove_script(&command, &expected))?;
    }
    Ok(inspect_installation(Some(&source), &command))
}

pub(in crate::commands) fn replace_link(
    source: &Path,
    command: &Path,
    expected: Option<&Path>,
) -> io::Result<()> {
    use std::os::unix::fs::symlink;

    if let Some(parent) = command.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(expected) = expected {
        remove_link(command, expected)?;
    }
    symlink(source, command)
}

pub(in crate::commands) fn remove_link(command: &Path, expected: &Path) -> io::Result<()> {
    if fs::read_link(command)? != expected {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "CLI link changed",
        ));
    }
    fs::remove_file(command)
}

pub(in crate::commands) fn is_pixelate_link(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "pixelate")
        && path
            .components()
            .any(|component| component.as_os_str() == "Pixelate.app")
}

fn link_script(source: &Path, command: &Path, expected: Option<&Path>) -> String {
    let install = format!("/bin/ln -s {} {}", quote(source), quote(command));
    expected.map_or_else(
        || {
            format!(
                "/bin/mkdir -p {} && {install}",
                quote(command.parent().unwrap_or(Path::new("/usr/local/bin")))
            )
        },
        |expected| format!("{} && {install}", verified_remove(command, expected)),
    )
}

fn remove_script(command: &Path, expected: &Path) -> String {
    verified_remove(command, expected)
}

fn verified_remove(command: &Path, expected: &Path) -> String {
    format!(
        "test \"$(/usr/bin/readlink {})\" = {} && /bin/rm {}",
        quote(command),
        quote(expected),
        quote(command)
    )
}

pub(in crate::commands) fn quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\"'\"'"))
}

#[cfg(target_os = "macos")]
fn authorize(script: String) -> CommandResult<()> {
    let output = std::process::Command::new("/usr/bin/osascript")
        .args([
            "-e",
            "on run argv",
            "-e",
            "do shell script (item 1 of argv) with administrator privileges",
            "-e",
            "end run",
            "--",
            &script,
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

#[cfg(not(target_os = "macos"))]
fn authorize(_script: String) -> CommandResult<()> {
    Err("CLI installation is available in the macOS app".to_owned())
}
