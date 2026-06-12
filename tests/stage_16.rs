use assert_cmd::Command;
use std::fs;

#[test]
fn test_stderr_redirection() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let err_file = "test_err.txt";

    // Clean up any old test files
    let _ = fs::remove_file(err_file);

    // Run a failing command and redirect stderr
    cmd.write_stdin(format!(
        "cd /directory_does_not_exist 2> {}\nexit 0\n",
        err_file
    ))
    .assert()
    .success();

    // Verify the error was written to the file
    let contents = fs::read_to_string(err_file).expect("Error file was not created");
    assert!(contents.contains("cd: /directory_does_not_exist: No such file or directory"));

    // Clean up
    let _ = fs::remove_file(err_file);
}
