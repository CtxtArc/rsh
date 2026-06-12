use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn test_background_process() {
    // Get the path to your compiled binary
    let exe = env!("CARGO_BIN_EXE_rsh");

    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn shell");

    let mut stdin = child.stdin.take().unwrap();
    let start = Instant::now();

    // Send the background command
    stdin
        .write_all(b"sleep 2 > /dev/null &\necho unblocked\nexit 0\n")
        .unwrap();
    drop(stdin);

    // Read the output
    let output = child.wait_with_output().expect("Failed to read stdout");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify output and timing
    assert!(stdout.contains("unblocked"));
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "Shell blocked on a background task! Elapsed: {:?}",
        start.elapsed()
    );
}
