use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_native_regex_operator() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let script = r#"
for VAR in "HELLO" "world" "RUST123" "12345"
do
    if [[ $VAR =~ ^[A-Z]+$ ]]
    then
        echo "$VAR is ALL CAPS"
    else
        if [[ $VAR =~ ^[0-9]+$ ]]
        then
            echo "$VAR is STRICTLY NUMBERS"
        else
            echo "$VAR is MIXED or LOWERCASE"
        fi
    fi
done
exit 0
"#;

    cmd.write_stdin(script)
        .assert()
        .success()
        .stdout(predicate::str::contains("HELLO is ALL CAPS"))
        .stdout(predicate::str::contains("12345 is STRICTLY NUMBERS"))
        .stdout(predicate::str::contains("world is MIXED or LOWERCASE"))
        .stdout(predicate::str::contains("RUST123 is MIXED or LOWERCASE"));
}
