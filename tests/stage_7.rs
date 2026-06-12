use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_stdout_redirect_overwrite() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let file = "out1.txt";
    let _ = fs::remove_file(file); // cleanup before

    cmd.write_stdin(format!(
        "echo \"first\" > {}\necho \"second\" > {}\nexit 0\n",
        file, file
    ))
    .assert()
    .success();

    let content = fs::read_to_string(file).unwrap();
    assert_eq!(content, "second\n");
    let _ = fs::remove_file(file); // cleanup after
}

#[test]
fn test_stdout_redirect_append() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let file = "out2.txt";
    let _ = fs::remove_file(file);

    cmd.write_stdin(format!(
        "echo \"first\" > {}\necho \"second\" >> {}\nexit 0\n",
        file, file
    ))
    .assert()
    .success();

    let content = fs::read_to_string(file).unwrap();
    assert_eq!(content, "first\nsecond\n");
    let _ = fs::remove_file(file);
}

#[test]
fn test_stdin_redirect() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let file = "in1.txt";
    fs::write(file, "hello from file\n").unwrap();

    cmd.write_stdin(format!("cat < {}\nexit 0\n", file))
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from file\n"));

    let _ = fs::remove_file(file);
}
