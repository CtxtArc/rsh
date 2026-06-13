use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_tokenizer_and_escape_edge_cases() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Using standard Rust string escaping to avoid raw string macro panics.
    // \n = newline, \" = quote, \\ = backslash
    let script = "\
echo \"hello world\" 'single space' escaped\\ space\n\
echo a#b # this is a comment\n\
echo \"#not a comment\"\n\
echo \"he said \\\"hello\\\"\"\n\
echo \"pipe test\" |\n\
grep \"pipe test\"\n\
if true; then\n\
    echo \"block test passed\"\n\
fi\n\
echo \"line 1\n\
line 2\"\n\
exit 0\n\
";

    cmd.write_stdin(script)
        .assert()
        .success()
        // 1. Verify quotes and escapes are evaluated properly
        .stdout(predicate::str::contains(
            "hello world single space escaped space",
        ))
        // 2. Verify inline comments are stripped, but word-hashes remain
        .stdout(
            predicate::str::contains("a#b")
                .and(predicate::str::contains("this is a comment").not()),
        )
        // 3. Verify comments inside quotes are printed
        .stdout(predicate::str::contains("#not a comment"))
        // 4. Verify escaped internal quotes
        .stdout(predicate::str::contains("he said \"hello\""))
        // 5. Verify the trailing pipe deferred execution
        .stdout(predicate::str::contains("pipe test"))
        // 6. Verify the AST block waited for the 'fi'
        .stdout(predicate::str::contains("block test passed"))
        // 7. Verify multi-line strings inside quotes
        .stdout(predicate::str::contains("line 1\nline 2"));
}
