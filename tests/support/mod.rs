use assert_cmd::Command;

/// Build a `Command` for the `lx` binary under test.
/// Isolated from the user's config by pointing LX_CONFIG and HOME
/// at locations that won't contain a config file.
pub fn lx() -> Command {
    let mut cmd = Command::cargo_bin("lx").expect("binary lx not found");
    cmd.env("LX_CONFIG", "/nonexistent")
        .env("HOME", "/nonexistent");
    cmd
}

/// Build a `Command` with colour forced off (for predictable output matching).
pub fn lx_no_colour() -> Command {
    let mut cmd = lx();
    cmd.arg("--colour=never");
    cmd
}
