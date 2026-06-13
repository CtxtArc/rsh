use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_ultimate_deployment_script() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Look at this! Raw newlines instead of semicolons.
    // This proves the Tokenizer natively understands multi-line scripts!
    let script = r#"
for DIR in src tests target
do 
    if ls $DIR 2> /dev/null
    then 
        echo "Found $DIR." 
    else 
        echo "Creating ${DIR:-fallback}..." 
        mkdir $DIR && echo "Success! Code: $?" || echo "Failed! Code: $?" 
    fi 
done
exit 0
"#;

    cmd.write_stdin(script)
        .assert()
        .success()
        // It should find the directories that exist in your cargo project
        .stdout(predicate::str::contains("Found src."))
        .stdout(predicate::str::contains("Found tests."));
}

#[test]
fn test_while_loop_ast() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Proves the AST successfully parses and evaluates `while` loops
    let script = r#"
while ls /directory_does_not_exist 2> /dev/null
do
    echo "FAIL: This should never print!"
done

echo "While loop safely bypassed!"
exit 0
"#;

    cmd.write_stdin(script)
        .assert()
        .success()
        // The loop body should NEVER run because the condition fails
        .stdout(predicate::str::contains("FAIL").not())
        // The rest of the script should continue normally
        .stdout(predicate::str::contains("While loop safely bypassed!"));
}
