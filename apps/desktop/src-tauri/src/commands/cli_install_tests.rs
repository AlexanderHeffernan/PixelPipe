use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

use tempfile::tempdir;

use super::cli_install::{
    CliInstallState,
    macos::{inspect_installation, is_pixelate_link, quote, remove_link, replace_link},
};

#[cfg(target_os = "linux")]
use super::cli_install::linux::{
    inspect_installation as inspect_linux_installation, install_copy, remove_copy,
};

#[test]
fn installs_repairs_and_removes_the_managed_link() {
    let fixture = tempdir().expect("temporary directory");
    let source = fixture.path().join("Pixelate.app/Contents/MacOS/pixelate");
    fs::create_dir_all(source.parent().expect("app directory")).expect("app directory");
    fs::write(&source, "cli").expect("CLI");
    let command = fixture.path().join("bin/pixelate");

    replace_link(&source, &command, None).expect("install CLI link");
    assert_eq!(fs::read_link(&command).expect("installed target"), source);

    let old = source.with_file_name("old-pixelate");
    fs::remove_file(&command).expect("remove installed link");
    symlink(&old, &command).expect("stale link");
    replace_link(&source, &command, Some(&old)).expect("repair CLI link");
    assert_eq!(fs::read_link(&command).expect("repaired target"), source);

    remove_link(&command, &source).expect("remove CLI link");
    assert!(!command.exists());
}

#[test]
fn distinguishes_installed_repairable_and_conflicting_commands() {
    let fixture = tempdir().expect("temporary directory");
    let app = fixture.path().join("Pixelate.app/Contents/MacOS");
    fs::create_dir_all(&app).expect("app directory");
    let source = app.join("pixelate");
    fs::write(&source, "cli").expect("CLI");
    let command = fixture.path().join("bin/pixelate");
    fs::create_dir_all(command.parent().expect("bin directory")).expect("bin directory");

    assert_eq!(
        inspect_installation(Some(&source), &command).state,
        CliInstallState::NotInstalled
    );
    symlink(&source, &command).expect("installed link");
    assert_eq!(
        inspect_installation(Some(&source), &command).state,
        CliInstallState::Installed
    );

    fs::remove_file(&command).expect("remove link");
    symlink(
        "/Users/alex/Applications/Pixelate.app/Contents/MacOS/pixelate",
        &command,
    )
    .expect("old link");
    assert_eq!(
        inspect_installation(Some(&source), &command).state,
        CliInstallState::NeedsRepair
    );

    fs::remove_file(&command).expect("remove link");
    fs::write(&command, "other command").expect("conflicting command");
    assert_eq!(
        inspect_installation(Some(&source), &command).state,
        CliInstallState::Conflict
    );
}

#[test]
fn only_recognizes_cli_links_inside_app_bundles() {
    assert!(is_pixelate_link(std::path::Path::new(
        "/Applications/Pixelate.app/Contents/MacOS/pixelate"
    )));
    assert!(!is_pixelate_link(std::path::Path::new(
        "/opt/homebrew/bin/pixelate"
    )));
    assert!(!is_pixelate_link(std::path::Path::new(
        "/Applications/Other.app/Contents/MacOS/pixelate"
    )));
}

#[test]
fn quotes_apostrophes_for_privileged_shell_commands() {
    assert_eq!(
        quote(std::path::Path::new("/Applications/Alex's App")),
        "'/Applications/Alex'\"'\"'s App'"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn installs_repairs_and_removes_the_managed_linux_command() {
    let fixture = tempdir().expect("temporary directory");
    let source = fixture.path().join("app/pixelate");
    fs::create_dir_all(source.parent().expect("app directory")).expect("app directory");
    fs::write(&source, "version one").expect("CLI");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).expect("executable CLI");
    let command = fixture.path().join("home/.local/bin/pixelate");
    let marker = fixture
        .path()
        .join("home/.local/share/pixelate/cli-install.sha256");

    install_copy(&source, &command, &marker).expect("install CLI copy");
    let installed = inspect_linux_installation(&source, &command, &marker);
    assert_eq!(installed.state, CliInstallState::Installed);
    assert!(installed.managed);
    assert_ne!(
        fs::metadata(&command)
            .expect("command metadata")
            .permissions()
            .mode()
            & 0o111,
        0
    );

    fs::write(&source, "version two").expect("updated CLI");
    assert_eq!(
        inspect_linux_installation(&source, &command, &marker).state,
        CliInstallState::NeedsRepair
    );
    install_copy(&source, &command, &marker).expect("repair CLI copy");
    assert_eq!(fs::read(&command).expect("installed CLI"), b"version two");

    remove_copy(&command, &marker).expect("remove CLI copy");
    assert!(!command.exists());
    assert!(!marker.exists());
}

#[cfg(target_os = "linux")]
#[test]
fn never_overwrites_an_unmanaged_linux_command() {
    let fixture = tempdir().expect("temporary directory");
    let source = fixture.path().join("app/pixelate");
    fs::create_dir_all(source.parent().expect("app directory")).expect("app directory");
    fs::write(&source, "pixelate").expect("CLI");
    let command = fixture.path().join("home/.local/bin/pixelate");
    fs::create_dir_all(command.parent().expect("bin directory")).expect("bin directory");
    fs::write(&command, "other command").expect("other command");
    let marker = fixture
        .path()
        .join("home/.local/share/pixelate/cli-install.sha256");

    let status = inspect_linux_installation(&source, &command, &marker);
    assert_eq!(status.state, CliInstallState::Conflict);
    assert!(!status.managed);
}
