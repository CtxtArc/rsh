use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_command_substitution_simple() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("echo \"Inner says: $(echo hello)\"\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Inner says: hello\n"));
}

#[test]
fn test_command_substitution_nested_logic() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // The inner command is a pipeline! Our subshell handles this effortlessly.
    cmd.write_stdin("echo \"Found: $(echo one | type cat)\"\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found: cat is"));
}

#[test]
fn test_command_substitution_strips_newlines() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // The inner command normally outputs "line1\nline2\n".
    // Substitution should capture it and strip the final newline.
    cmd.write_stdin("echo $(echo line1 && echo line2)\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("line1\nline2\n"));
}
