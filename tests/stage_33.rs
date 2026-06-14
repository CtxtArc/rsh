use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_all_native_operators() {
    // 1. Setup a temporary filesystem for the file tests
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_file.txt");
    fs::write(&file_path, "hello world").unwrap();

    let dir_str = dir.path().to_str().unwrap();
    let file_str = file_path.to_str().unwrap();

    // 2. Prepare the shell
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // 3. The Ultimate Test Script
    // We use if/else statements. If the operator works, it prints PASS. If it fails, it prints FAIL.
    let script = format!(
        r#"
if [ -z "" ]; then echo "PASS_Z"; else echo "FAIL_Z"; fi
if [ -n "hello" ]; then echo "PASS_N"; else echo "FAIL_N"; fi
if [ "a" = "a" ]; then echo "PASS_EQ_STR"; else echo "FAIL_EQ_STR"; fi
if [ "a" != "b" ]; then echo "PASS_NEQ_STR"; else echo "FAIL_NEQ_STR"; fi

if [ 10 -eq 10 ]; then echo "PASS_EQ_INT"; else echo "FAIL_EQ_INT"; fi
if [ 10 -ne 5 ]; then echo "PASS_NE_INT"; else echo "FAIL_NE_INT"; fi
if [ 15 -gt 10 ]; then echo "PASS_GT"; else echo "FAIL_GT"; fi
if [ 5 -lt 10 ]; then echo "PASS_LT"; else echo "FAIL_LT"; fi
if [ 10 -ge 10 ]; then echo "PASS_GE"; else echo "FAIL_GE"; fi
if [ 5 -le 5 ]; then echo "PASS_LE"; else echo "FAIL_LE"; fi

if [ -e "{file_str}" ]; then echo "PASS_E_FILE"; else echo "FAIL_E_FILE"; fi
if [ -f "{file_str}" ]; then echo "PASS_F_FILE"; else echo "FAIL_F_FILE"; fi
if [ -d "{dir_str}" ]; then echo "PASS_D_DIR"; else echo "FAIL_D_DIR"; fi

if [ -f "{dir_str}" ]; then echo "FAIL_F_DIR"; else echo "PASS_F_DIR"; fi
if [ -d "{file_str}" ]; then echo "FAIL_D_FILE"; else echo "PASS_D_FILE"; fi
if [ -e "/path/that/definitely/does/not/exist/123" ]; then echo "FAIL_E_NONE"; else echo "PASS_E_NONE"; fi

exit 0
"#
    );

    // 4. Execute and Assert
    cmd.write_stdin(script)
        .assert()
        .success()
        // Check Strings
        .stdout(predicate::str::contains("PASS_Z"))
        .stdout(predicate::str::contains("PASS_N"))
        .stdout(predicate::str::contains("PASS_EQ_STR"))
        .stdout(predicate::str::contains("PASS_NEQ_STR"))
        // Check Integers
        .stdout(predicate::str::contains("PASS_EQ_INT"))
        .stdout(predicate::str::contains("PASS_NE_INT"))
        .stdout(predicate::str::contains("PASS_GT"))
        .stdout(predicate::str::contains("PASS_LT"))
        .stdout(predicate::str::contains("PASS_GE"))
        .stdout(predicate::str::contains("PASS_LE"))
        // Check Files
        .stdout(predicate::str::contains("PASS_E_FILE"))
        .stdout(predicate::str::contains("PASS_F_FILE"))
        .stdout(predicate::str::contains("PASS_D_DIR"))
        .stdout(predicate::str::contains("PASS_F_DIR"))
        .stdout(predicate::str::contains("PASS_D_FILE"))
        .stdout(predicate::str::contains("PASS_E_NONE"))
        // CRITICAL: Ensure absolutely zero "FAIL" strings were printed
        .stdout(predicate::str::contains("FAIL").not());
}
