use std::{
    path::Path,
    process::{Command, Stdio},
};

use super::terminal::{fallback_shell, shell_configuration};

#[test]
fn macos_zsh_loads_user_environment_then_restores_pixelate_cli() {
    let (arguments, path_setup) = shell_configuration(Path::new("/bin/zsh"), "macos");

    assert_eq!(arguments, ["-l", "-i"]);
    assert!(path_setup.is_some_and(|setup| setup.contains("PIXELATE_CLI_DIR")));
}

#[test]
fn macos_bash_loads_user_environment_then_restores_pixelate_cli() {
    let (arguments, path_setup) = shell_configuration(Path::new("/bin/bash"), "macos");

    assert_eq!(arguments, ["--login", "-i"]);
    assert!(path_setup.is_some_and(|setup| setup.contains("PIXELATE_CLI_DIR")));
}

#[test]
fn path_setup_prepends_cli_directory_to_replaced_user_path() {
    let (_, Some(path_setup)) = shell_configuration(Path::new("/bin/bash"), "linux") else {
        panic!("bash must restore the bundled CLI path");
    };
    let status = Command::new("/bin/bash")
        .arg("-c")
        .arg(format!(
            "{path_setup}test \"$PATH\" = \"$PIXELATE_CLI_DIR:/usr/bin\""
        ))
        .env("PATH", "/usr/bin")
        .env(
            "PIXELATE_CLI_DIR",
            "/Applications/Pixelate.app/Contents/MacOS",
        )
        .stdout(Stdio::null())
        .status()
        .expect("run path setup");

    assert!(status.success());
}

#[test]
fn other_shells_keep_their_native_terminal_startup_behavior() {
    let (arguments, path_setup) = shell_configuration(Path::new("/opt/homebrew/bin/fish"), "macos");

    assert!(arguments.is_empty());
    assert!(path_setup.is_none());
}

#[test]
fn linux_shells_restore_the_cli_after_user_startup_files() {
    for shell in ["bash", "zsh", "sh", "fish"] {
        let (arguments, path_setup) =
            shell_configuration(Path::new(&format!("/usr/bin/{shell}")), "linux");
        assert!(arguments.is_empty());
        assert!(path_setup.is_some_and(|setup| setup.contains("PIXELATE_CLI_DIR")));
    }
}

#[test]
fn uses_platform_native_fallback_shells() {
    assert_eq!(fallback_shell("macos"), "/bin/zsh");
    assert_eq!(fallback_shell("linux"), "/bin/sh");
}
