use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_strict_quoting_and_escaping() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin(
        "export MY_VAR=Secret\n\
         echo \"Double: $MY_VAR\"\n\
         echo 'Single: $MY_VAR'\n\
         echo Escaped\\ Space\n\
         exit 0\n",
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Double: Secret")) // Double quotes expand variables
    .stdout(predicate::str::contains("Single: $MY_VAR")) // Single quotes prevent expansion
    .stdout(predicate::str::contains("Escaped Space")); // Backslash prevents word splitting
}
