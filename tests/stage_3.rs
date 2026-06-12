use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_type_builtin_echo() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("type echo\nexit 0\n")
        .assert()
        .stdout(predicate::str::contains("echo is a shell builtin\n"));
}

#[test]
fn test_type_builtin_exit() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("type exit\nexit 0\n")
        .assert()
        .stdout(predicate::str::contains("exit is a shell builtin\n"));
}

#[test]
fn test_type_builtin_type() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("type type\nexit 0\n")
        .assert()
        .stdout(predicate::str::contains("type is a shell builtin\n"));
}

#[test]
fn test_type_unknown_command() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("type randomcmd\nexit 0\n")
        .assert()
        .stdout(predicate::str::contains("randomcmd: not found\n"));
}

#[test]
fn test_type_external_command_ls() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // 'ls' should be found in the PATH (usually /bin/ls or /usr/bin/ls)
    cmd.write_stdin("type ls\nexit 0\n")
        .assert()
        .stdout(predicate::str::is_match(r"ls is .+/ls\n").unwrap());
}

#[test]
fn test_type_external_command_cat() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("type cat\nexit 0\n")
        .assert()
        .stdout(predicate::str::is_match(r"cat is .+/cat\n").unwrap());
}
