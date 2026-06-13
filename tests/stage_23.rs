use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_control_flow_ast() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // We will test 4 distinct AST pathways:
    // 1. A successful condition (runs THEN)
    // 2. A failing condition with an ELSE (runs ELSE)
    // 3. A failing condition with NO ELSE (runs nothing, succeeds)
    // 4. A nested IF statement (proves recursive AST evaluation)
    cmd.write_stdin(
        "if echo test > /dev/null ; then echo \"Branch 1: THEN\" ; else echo \"FAIL\" ; fi\n\
         if ls /does_not_exist 2> /dev/null ; then echo \"FAIL\" ; else echo \"Branch 2: ELSE\" ; fi\n\
         if ls /does_not_exist 2> /dev/null ; then echo \"FAIL\" ; fi\n\
         if echo a > /dev/null ; then if echo b > /dev/null ; then echo \"Branch 4: NESTED\" ; fi ; fi\n\
         exit 0\n"
    )
    .assert()
    .success()
    // Test 1: The true branch must execute
    .stdout(predicate::str::contains("Branch 1: THEN"))
    // Test 2: The false branch must execute
    .stdout(predicate::str::contains("Branch 2: ELSE"))
    // Test 3: The nested true branch must execute
    .stdout(predicate::str::contains("Branch 4: NESTED"))
    // Ensure the failing branches NEVER executed
    .stdout(predicate::str::contains("FAIL").not());
}
