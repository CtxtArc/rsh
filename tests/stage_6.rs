use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_echo_with_single_quotes() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("echo 'hello     world'\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello     world\n"));
}

#[test]
fn test_echo_with_double_quotes() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("echo \"rust    is    awesome\"\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("rust    is    awesome\n"));
}

#[test]
fn test_echo_mixed_quotes() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Tests multiple arguments with different quote types
    cmd.write_stdin("echo 'single   quote' \"double   quote\"\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("single   quote double   quote\n"));
}
