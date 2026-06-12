use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_startup_config() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Create a temporary "home" directory so we don't mess with the real one
    let fake_home = tempfile::tempdir().unwrap();
    let rc_path = fake_home.path().join(".rshrc");

    // Write a startup config with an export and an alias
    fs::write(
        &rc_path,
        "export TEST_RC_VAR=\"bootstrapped\"\nalias rc_echo=\"echo rc_loaded\"\n",
    )
    .unwrap();

    // Run the shell, overriding the HOME environment variable to trick it!
    cmd.env("HOME", fake_home.path())
        .write_stdin("echo $TEST_RC_VAR\nrc_echo\nexit 0\n")
        .assert()
        .success()
        // Verify the environment variable was loaded
        .stdout(predicate::str::contains("bootstrapped"))
        // Verify the alias was loaded and expanded
        .stdout(predicate::str::contains("rc_loaded"));
}
