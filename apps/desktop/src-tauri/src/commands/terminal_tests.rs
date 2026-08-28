use std::path::Path;

use super::terminal::macos_shell_arguments;

#[test]
fn macos_zsh_loads_login_and_interactive_startup_files() {
    assert_eq!(macos_shell_arguments(Path::new("/bin/zsh")), ["-l", "-i"]);
}

#[test]
fn macos_bash_loads_login_and_interactive_startup_files() {
    assert_eq!(
        macos_shell_arguments(Path::new("/bin/bash")),
        ["--login", "-i"]
    );
}

#[test]
fn other_shells_keep_their_native_terminal_startup_behavior() {
    assert!(macos_shell_arguments(Path::new("/opt/homebrew/bin/fish")).is_empty());
}
