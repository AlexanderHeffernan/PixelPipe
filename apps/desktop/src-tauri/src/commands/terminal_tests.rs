use std::{
    path::Path,
    process::{Command, Stdio},
};

use super::terminal::macos_shell_configuration;

#[test]
fn macos_zsh_loads_user_environment_then_restores_pixelate_cli() {
    let (arguments, path_setup) = macos_shell_configuration(Path::new("/bin/zsh"));

    assert_eq!(arguments, ["-l", "-i"]);
    assert!(path_setup.is_some_and(|setup| setup.contains("PIXELATE_CLI_DIR")));
}

#[test]
fn macos_bash_loads_user_environment_then_restores_pixelate_cli() {
    let (arguments, path_setup) = macos_shell_configuration(Path::new("/bin/bash"));

    assert_eq!(arguments, ["--login", "-i"]);
    assert!(path_setup.is_some_and(|setup| setup.contains("PIXELATE_CLI_DIR")));
}

#[test]
fn path_setup_prepends_cli_directory_to_replaced_user_path() {
    let (_, Some(path_setup)) = macos_shell_configuration(Path::new("/bin/bash")) else {
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
    let (arguments, path_setup) = macos_shell_configuration(Path::new("/opt/homebrew/bin/fish"));

    assert!(arguments.is_empty());
    assert!(path_setup.is_none());
}
