use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_parameter_expansion() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin(
        "ls /directory_does_not_exist 2> /dev/null\n\
         echo \"Exit Code: $?\"\n\
         echo \"Default: ${MISSING_VAR:-FallbackValue}\"\n\
         export EXISTING_VAR=\"FoundMe\"\n\
         echo \"Exists: ${EXISTING_VAR:-FallbackValue}\"\n\
         exit 0\n",
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Exit Code: 2").or(predicate::str::contains("Exit Code: 1"))) // ls usually exits with 1 or 2 on failure
    .stdout(predicate::str::contains("Default: FallbackValue"))
    .stdout(predicate::str::contains("Exists: FoundMe"));
}
