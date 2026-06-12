// tests/stage_1.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_repl_prints_prompt_and_exits() {
    // This looks for the binary compiled by cargo
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // We feed it the "exit 0" command via standard input
    cmd.write_stdin("exit 0\n")
        .assert()
        .success() // Expects exit code 0
        .stdout(predicate::str::starts_with("$ "));
}

#[test]
fn test_exit_with_custom_code() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // We feed it "exit 42"
    cmd.write_stdin("exit 42\n").assert().code(42); // Expects exit code 42
}
