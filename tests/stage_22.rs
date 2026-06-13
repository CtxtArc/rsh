use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_arithmetic_expansion() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    cmd.write_stdin(
        "echo \"Basic: $(( 5 + 5 ))\"\n\
         echo \"Order: $(( 5 + 10 * 2 ))\"\n\
         echo \"Parens: $(( (5 + 10) * 2 ))\"\n\
         exit 0\n",
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Basic: 10"))
    .stdout(predicate::str::contains("Order: 25")) // Proves order of operations!
    .stdout(predicate::str::contains("Parens: 30")); // Proves parenthesis isolation!
}
