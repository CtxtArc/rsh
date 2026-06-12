use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn test_background_process() {
    let exe = env!("CARGO_BIN_EXE_rsh");

    // 1. Create a fake home directory to prevent history file lock contention!
    let fake_home = tempfile::tempdir().unwrap();

    let mut child = Command::new(exe)
        .env("HOME", fake_home.path()) // 2. Override HOME for this process
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn shell");

    let mut stdin = child.stdin.take().unwrap();
    let start = Instant::now();

    // 3. Send the background command, redirecting BOTH stdout and stderr
    stdin
        .write_all(b"sleep 2 > /dev/null 2> /dev/null &\necho unblocked\nexit 0\n")
        .unwrap();
    drop(stdin);

    let output = child.wait_with_output().expect("Failed to read stdout");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("unblocked"));
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "Shell blocked on a background task! Elapsed: {:?}",
        start.elapsed()
    );
}
