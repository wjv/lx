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
