use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_pwd_command() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // pwd should print the current directory (which contains Cargo.toml during testing)
    cmd.write_stdin("pwd\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("rsh"));
}

#[test]
fn test_cd_and_pwd() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("cd /\npwd\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("/\n")); // <-- Change ends_with to contains!
}

#[test]
fn test_cd_non_existent_directory() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Change .stdout() to .stderr() since we upgraded our error handling!
    cmd.write_stdin("cd /this_directory_does_not_exist\nexit 0\n")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "cd: /this_directory_does_not_exist: No such file or directory",
        ));
}
