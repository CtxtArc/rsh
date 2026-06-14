use assert_cmd::Command;
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_readjson_builtin() {
    // 1. Create a temporary JSON file
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"TEST_KEY": "hello_world", "TEST_NUM": "42"}}"#).unwrap();
    let path = file.path().to_str().unwrap();

    // 2. Invoke rsh with the readjson command
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Pass the command to our shell
    let script = format!("readjson {} && echo $TEST_KEY $TEST_NUM", path);

    cmd.write_stdin(script)
        .assert()
        .success()
        .stdout(predicates::str::contains("hello_world 42"));
}
