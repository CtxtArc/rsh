use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_custom_super_operators() {
    // 1. Setup a temporary file for the -fcontains test
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.env");
    fs::write(&file_path, "PORT=8080\nDEBUG=true\nSECRET=12345").unwrap();

    let file_str = file_path.to_str().unwrap();

    // 2. Prepare the shell
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // 3. The Test Script (No comments to avoid REPL parsing issues!)
    let script = format!(
        r#"
if [ "hello world" -contains "world" ]; then echo "PASS_CONTAINS"; else echo "FAIL_CONTAINS"; fi
if [ "hello world" -contains "xyz" ]; then echo "FAIL_NCONTAINS"; else echo "PASS_NCONTAINS"; fi

if [ "hello world" -starts "hello" ]; then echo "PASS_STARTS"; else echo "FAIL_STARTS"; fi
if [ "hello world" -ends "world" ]; then echo "PASS_ENDS"; else echo "FAIL_ENDS"; fi

if [ -isint "42" ]; then echo "PASS_ISINT_POS"; else echo "FAIL_ISINT_POS"; fi
if [ -isint "-99" ]; then echo "PASS_ISINT_NEG"; else echo "FAIL_ISINT_NEG"; fi
if [ -isint "3.14" ]; then echo "FAIL_ISINT_FLOAT"; else echo "PASS_ISINT_FLOAT"; fi
if [ -isint "abc" ]; then echo "FAIL_ISINT_STR"; else echo "PASS_ISINT_STR"; fi

if [ -isnum "3.1415" ]; then echo "PASS_ISNUM_FLOAT"; else echo "FAIL_ISNUM_FLOAT"; fi
if [ -isnum "-0.001" ]; then echo "PASS_ISNUM_NEG"; else echo "FAIL_ISNUM_NEG"; fi
if [ -isnum "not_a_number" ]; then echo "FAIL_ISNUM_STR"; else echo "PASS_ISNUM_STR"; fi

if [ "{file_str}" -fcontains "DEBUG=true" ]; then echo "PASS_FCONTAINS"; else echo "FAIL_FCONTAINS"; fi
if [ "{file_str}" -fcontains "MISSING=yes" ]; then echo "FAIL_FNCONTAINS"; else echo "PASS_FNCONTAINS"; fi

exit 0
"#
    );

    // 4. Execute and Assert
    cmd.write_stdin(script)
        .assert()
        .success()
        // Check Substrings
        .stdout(predicate::str::contains("PASS_CONTAINS"))
        .stdout(predicate::str::contains("PASS_NCONTAINS"))
        .stdout(predicate::str::contains("PASS_STARTS"))
        .stdout(predicate::str::contains("PASS_ENDS"))
        // Check Type Inference
        .stdout(predicate::str::contains("PASS_ISINT_POS"))
        .stdout(predicate::str::contains("PASS_ISINT_NEG"))
        .stdout(predicate::str::contains("PASS_ISINT_FLOAT"))
        .stdout(predicate::str::contains("PASS_ISINT_STR"))
        .stdout(predicate::str::contains("PASS_ISNUM_FLOAT"))
        .stdout(predicate::str::contains("PASS_ISNUM_NEG"))
        .stdout(predicate::str::contains("PASS_ISNUM_STR"))
        // Check File Contents
        .stdout(predicate::str::contains("PASS_FCONTAINS"))
        .stdout(predicate::str::contains("PASS_FNCONTAINS"))
        // CRITICAL: Ensure absolutely zero "FAIL" strings were printed
        .stdout(predicate::str::contains("FAIL").not());
}
