use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_heredoc() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // We feed the shell a heredoc command via stdin
    let script = "\
cat << EOF
line1
line2
EOF
exit 0
";

    cmd.write_stdin(script)
        .assert()
        .success()
        // Ensure both lines are passed through and printed
        .stdout(predicate::str::contains("line1\nline2"));
}
