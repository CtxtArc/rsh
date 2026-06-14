use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_time_readjson_massive_file() {
    // 1. Generate a massive JSON file (e.g., 50,000 keys)
    let mut file = NamedTempFile::new().unwrap();

    write!(file, "{{").unwrap();
    let num_keys = 50_000;
    for i in 0..num_keys {
        write!(file, "\"CONFIG_KEY_{}\": \"config_value_{}\"", i, i).unwrap();
        if i < num_keys - 1 {
            write!(file, ",").unwrap();
        }
    }
    writeln!(file, "}}").unwrap();
    let path = file.path().to_str().unwrap();

    // 2. Invoke the shell
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Pass the `time` keyword wrapping our `readjson` builtin
    let script = format!("time readjson {}\nexit 0\n", path);

    cmd.write_stdin(script)
        .assert()
        .success()
        // 3. Verify the time keyword intercepted it and wrote to stderr
        .stderr(predicate::str::contains("real\t"))
        .stderr(predicate::str::contains("user\t"))
        .stderr(predicate::str::contains("sys\t"));
}

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
