use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs::File;
use std::io::Write;

#[test]
fn test_source_builtin() {
    // 1. Create a file containing a function definition
    let script_path = env::temp_dir().join("test_lib.rsh");
    let mut file = File::create(&script_path).unwrap();
    file.write_all(b"loaded_func() { echo 'Loaded successfully'; }")
        .unwrap();

    // 2. Run a command that sources the file and then calls the function
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // We source the file and then execute the function defined inside it
    let script = format!("source {} ; loaded_func", script_path.to_str().unwrap());

    cmd.write_stdin(script)
        .assert()
        .success()
        .stdout(predicate::str::contains("Loaded successfully"));
}
