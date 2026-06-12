use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_glob_expansion() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Setup: Create some dummy files to match against
    let _ = fs::write("glob_test_1.txt", "1");
    let _ = fs::write("glob_test_2.txt", "2");
    let _ = fs::write("glob_test_ignore.rs", "3");

    cmd.write_stdin("echo glob_test_*.txt\nexit 0\n")
        .assert()
        .success()
        // It should match the .txt files in alphabetical order, and ignore the .rs file
        .stdout(predicate::str::contains("glob_test_1.txt glob_test_2.txt"));

    // Cleanup
    let _ = fs::remove_file("glob_test_1.txt");
    let _ = fs::remove_file("glob_test_2.txt");
    let _ = fs::remove_file("glob_test_ignore.rs");
}

#[test]
fn test_glob_no_match() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // If no files match, it should print the literal asterisk string
    cmd.write_stdin("echo this_file_does_not_exist_*.xyz\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("this_file_does_not_exist_*.xyz\n"));
}
