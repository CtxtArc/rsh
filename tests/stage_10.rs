use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_logical_and() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // The first command succeeds, so the second should run.
    cmd.write_stdin("echo first && echo second\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("first\nsecond\n"));
}

#[test]
fn test_logical_and_short_circuit() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // The first command fails, so the second SHOULD NOT run.
    cmd.write_stdin("ls /directory_does_not_exist && echo second\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("second").not());
}

#[test]
fn test_logical_or() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // The first command fails, so the second SHOULD run.
    cmd.write_stdin("ls /directory_does_not_exist || echo fallback\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("fallback\n"));
}

#[test]
fn test_logical_or_short_circuit() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // The first command succeeds, so the second SHOULD NOT run.
    cmd.write_stdin("echo success || echo fallback\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("fallback").not());
}
