use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_simple_pipeline() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // echo prints to stdout, the pipe catches it, and cat prints it back out
    cmd.write_stdin("echo \"pipeline works\" | cat\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("pipeline works\n"));
}

#[test]
fn test_long_pipeline() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // echo -> cat -> grep -> terminal
    cmd.write_stdin("echo \"pipeline works\npipeline fails\" | cat | grep works\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("pipeline works\n"));
}
