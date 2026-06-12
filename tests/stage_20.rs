use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_job_control_state() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // 1. Create a fake home directory to prevent history file lock contention
    let fake_home = tempfile::tempdir().unwrap();

    // 2. Run the shell, launch a background job, check jobs, and exit
    cmd.env("HOME", fake_home.path())
        .write_stdin("sleep 2 &\njobs\nexit 0\n")
        .assert()
        .success()
        // Verify the background job was assigned ID 1 and prints its PID
        .stdout(predicate::str::contains("[1]"))
        // Verify the `jobs` command lists it as Running
        .stdout(predicate::str::contains("Running"))
        .stdout(predicate::str::contains("sleep 2"));
}
