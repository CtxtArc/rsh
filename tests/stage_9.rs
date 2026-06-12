use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_export_sets_environment_variable() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // We export a variable, then spawn a standard Unix shell to print it
    // Because child processes inherit the environment, 'sh' will see 'MY_VAR'
    cmd.write_stdin("export MY_VAR=codecrafters_rust\nsh -c 'echo $MY_VAR'\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("codecrafters_rust"));
}

#[test]
fn test_tilde_expansion_standalone() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let home_dir = std::env::var("HOME").unwrap_or_default();

    cmd.write_stdin("echo ~\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(home_dir));
}

#[test]
fn test_tilde_expansion_with_path() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let home_dir = std::env::var("HOME").unwrap_or_default();

    // Test that ~/Documents properly expands to /home/user/Documents
    let expected_path = format!("{}/Documents", home_dir);

    cmd.write_stdin("echo ~/Documents\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(expected_path));
}
