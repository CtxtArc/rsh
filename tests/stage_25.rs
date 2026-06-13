use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_user_defined_functions() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let script = r#"
# Test 1: Function definition and positional arguments
greet() {
    echo "Welcome, $1! Your role is $2."
}
greet "Alice" "Admin"

# Test 2: Function overriding (redefinition)
greet() {
    echo "OVERRIDDEN! Goodbye $1."
}
greet "Alice"

# Test 3: Multi-line function with nested AST logic
complex_func() {
    for X in 1 2
    do
        echo "Loop $X inside function: arg is $1"
    done
}
complex_func "Magic"

exit 0
"#;

    cmd.write_stdin(script)
        .assert()
        .success()
        // Verify Test 1 (Args)
        .stdout(predicate::str::contains(
            "Welcome, Alice! Your role is Admin.",
        ))
        // Verify Test 2 (Override)
        .stdout(predicate::str::contains("OVERRIDDEN! Goodbye Alice."))
        // Verify Test 3 (Complex nested AST)
        .stdout(predicate::str::contains(
            "Loop 1 inside function: arg is Magic",
        ))
        .stdout(predicate::str::contains(
            "Loop 2 inside function: arg is Magic",
        ));
}
