use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_alias_expansion() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Set an alias, use it, and exit
    cmd.write_stdin("alias greet=\"echo hello from alias\"\ngreet\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello from alias"));
}
