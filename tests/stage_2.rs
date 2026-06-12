use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_echo_single_word() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("echo hello\nexit 0\n")
        .assert()
        .stdout(predicate::str::contains("hello\n"));
}

#[test]
fn test_echo_multiple_words() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("echo hello world from rust\nexit 0\n")
        .assert()
        .stdout(predicate::str::contains("hello world from rust\n"));
}
