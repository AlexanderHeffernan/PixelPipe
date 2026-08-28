use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use super::{CliInstallState, CliInstallStatus, status};
use crate::commands::CommandResult;

struct InstallPaths {
    command: PathBuf,
    marker: PathBuf,
}

pub(super) fn current_status(source: Option<&Path>) -> CliInstallStatus {
    let Some(source) = source else {
        return status(
            CliInstallState::Unavailable,
            PathBuf::from("pixelate"),
            false,
        );
    };
    if is_system_command(source) {
        return status(CliInstallState::Installed, source.to_path_buf(), false);
    }
    let Some(paths) = install_paths() else {
        return status(
            CliInstallState::Unavailable,
            PathBuf::from("~/.local/bin/pixelate"),
            false,
        );
    };
    inspect_installation(source, &paths.command, &paths.marker)
}

pub(super) fn install(source: Option<PathBuf>) -> CommandResult<CliInstallStatus> {
    let source = source.ok_or_else(|| "the bundled Pixelate CLI is unavailable".to_owned())?;
    if is_system_command(&source) {
        return Ok(status(CliInstallState::Installed, source, false));
    }
    let paths = install_paths().ok_or_else(|| "HOME is unavailable".to_owned())?;
    let before = inspect_installation(&source, &paths.command, &paths.marker);
    match before.state {
        CliInstallState::Installed => return Ok(before),
        CliInstallState::Conflict => {
            return Err(format!(
                "another command already exists at {}",
                paths.command.display()
            ));
        }
        CliInstallState::Unavailable => return Err("CLI installation is unavailable".to_owned()),
        CliInstallState::NotInstalled | CliInstallState::NeedsRepair => {}
    }
    install_copy(&source, &paths.command, &paths.marker).map_err(|error| error.to_string())?;
    let after = inspect_installation(&source, &paths.command, &paths.marker);
    (after.state == CliInstallState::Installed)
        .then_some(after)
        .ok_or_else(|| "Pixelate CLI installation did not complete".to_owned())
}

pub(super) fn uninstall(source: Option<PathBuf>) -> CommandResult<CliInstallStatus> {
    let source = source.ok_or_else(|| "the bundled Pixelate CLI is unavailable".to_owned())?;
    if is_system_command(&source) {
        return Err("the Pixelate command is managed by the system package".to_owned());
    }
    let paths = install_paths().ok_or_else(|| "HOME is unavailable".to_owned())?;
    let before = inspect_installation(&source, &paths.command, &paths.marker);
    if before.state == CliInstallState::NotInstalled {
        return Ok(before);
    }
    if !before.managed
        || !matches!(
            before.state,
            CliInstallState::Installed | CliInstallState::NeedsRepair
        )
    {
        return Err(format!(
            "Pixelate does not manage the command at {}",
            paths.command.display()
        ));
    }
    remove_copy(&paths.command, &paths.marker).map_err(|error| error.to_string())?;
    Ok(inspect_installation(&source, &paths.command, &paths.marker))
}

fn install_paths() -> Option<InstallPaths> {
    let home = env::var_os("HOME").map(PathBuf::from)?;
    Some(InstallPaths {
        command: home.join(".local/bin/pixelate"),
        marker: home.join(".local/share/pixelate/cli-install.sha256"),
    })
}

fn is_system_command(source: &Path) -> bool {
    matches!(source.parent(), Some(parent) if parent == Path::new("/usr/bin") || parent == Path::new("/usr/local/bin"))
}

pub(in crate::commands) fn inspect_installation(
    source: &Path,
    command: &Path,
    marker: &Path,
) -> CliInstallStatus {
    let metadata = match fs::symlink_metadata(command) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return status(CliInstallState::NotInstalled, command.to_path_buf(), true);
        }
        Err(_) => return status(CliInstallState::Conflict, command.to_path_buf(), false),
        Ok(metadata) => metadata,
    };
    if !metadata.file_type().is_file() {
        return status(CliInstallState::Conflict, command.to_path_buf(), false);
    }
    let Ok(command_hash) = file_hash(command) else {
        return status(CliInstallState::Conflict, command.to_path_buf(), false);
    };
    let marker_hash = fs::read_to_string(marker)
        .ok()
        .map(|value| value.trim().to_owned());
    let managed = marker_hash.as_deref() == Some(&command_hash);
    let state = match file_hash(source) {
        Ok(source_hash) if source_hash == command_hash => CliInstallState::Installed,
        Ok(_) if managed => CliInstallState::NeedsRepair,
        _ => CliInstallState::Conflict,
    };
    status(state, command.to_path_buf(), managed)
}

pub(in crate::commands) fn install_copy(
    source: &Path,
    command: &Path,
    marker: &Path,
) -> io::Result<()> {
    let hash = file_hash(source)?;
    let command_parent = command
        .parent()
        .ok_or_else(|| io::Error::other("command has no parent"))?;
    let marker_parent = marker
        .parent()
        .ok_or_else(|| io::Error::other("marker has no parent"))?;
    fs::create_dir_all(command_parent)?;
    fs::create_dir_all(marker_parent)?;
    let suffix = std::process::id();
    let command_temp = command_parent.join(format!(".pixelate-install-{suffix}"));
    let marker_temp = marker_parent.join(format!(".cli-install-{suffix}"));
    let result = (|| {
        fs::copy(source, &command_temp)?;
        fs::set_permissions(&command_temp, fs::metadata(source)?.permissions())?;
        fs::write(&marker_temp, format!("{hash}\n"))?;
        fs::rename(&command_temp, command)?;
        fs::rename(&marker_temp, marker)
    })();
    if result.is_err() {
        let _ = fs::remove_file(command_temp);
        let _ = fs::remove_file(marker_temp);
    }
    result
}

pub(in crate::commands) fn remove_copy(command: &Path, marker: &Path) -> io::Result<()> {
    let expected = fs::read_to_string(marker)?;
    if file_hash(command)? != expected.trim() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "CLI command changed",
        ));
    }
    fs::remove_file(command)?;
    fs::remove_file(marker)
}

fn file_hash(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}
