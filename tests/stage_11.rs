use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_for_loop_simple() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin("for X in rust is awesome do echo $X done\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("rust\nis\nawesome\n"));
}

#[test]
fn test_for_loop_with_logic() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Loop through numbers, but only the `echo` commands will succeed
    cmd.write_stdin("for NUM in 1 2 do echo iteration && echo $NUM done\nexit 0\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("iteration\n1\niteration\n2\n"));
}
