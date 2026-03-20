use assert_cmd::Command;

/// Build a `Command` for the `lx` binary under test.
pub fn lx() -> Command {
    Command::cargo_bin("lx").expect("binary lx not found")
}

/// Build a `Command` with colour forced off (for predictable output matching).
pub fn lx_no_colour() -> Command {
    let mut cmd = lx();
    cmd.arg("--colour=never");
    cmd
}

/// Return the primary group name of the current user.
/// Used in tests that check for the group column being present.
pub fn current_group() -> String {
    use std::process::Command as StdCommand;
    let output = StdCommand::new("id")
        .arg("-gn")
        .output()
        .expect("failed to run id -gn");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
