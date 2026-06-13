use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs::File;
use std::io::Write;

#[test]
fn test_script_file_execution() {
    // 1. Dynamically create a temporary script file
    let script_path = env::temp_dir().join("test_script.rsh");
    let mut file = File::create(&script_path).unwrap();

    // Write a script that tests comments, logic, and a specific exit code
    let script_content = r#"#!/usr/bin/env rsh
# This is a comment that should be ignored
echo "Executing from file!"
if echo true > /dev/null ; then
    exit 42
else
    exit 1
fi
"#;
    file.write_all(script_content.as_bytes()).unwrap();

    // 2. Tell the `rsh` binary to execute the file: `rsh /path/to/test_script.rsh`
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    cmd.arg(script_path.to_str().unwrap())
        .assert()
        // We expect it to "fail" from the OS perspective because it exits with 42
        .failure()
        .code(42)
        .stdout(predicate::str::contains("Executing from file!"));
}
