use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_stream_merging() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // We purposely run 'cat' on a fake file.
    // Normally, this writes to stderr and 'grep' would receive absolutely nothing.
    // By adding 2>&1, the error flows into the pipe, and 'grep' will catch it and print it!
    let script = "\
cat /this_file_does_not_exist 2>&1 | grep \"No such file\"
exit 0
";

    cmd.write_stdin(script)
        .assert()
        .success()
        // If the stream merge worked, grep will successfully output the matched error text
        .stdout(predicate::str::contains("No such file"));
}
