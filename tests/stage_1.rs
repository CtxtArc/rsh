use assert_cmd::Command;

#[test]
fn test_repl_prints_prompt_and_exits() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // In Stage 1 we checked for the "$ " prompt.
    // Now that we use rustyline, prompts are hidden in non-TTY test pipes.
    // We just verify that sending 'exit 0' successfully terminates the shell.
    cmd.write_stdin("exit 0\n").assert().success();
}
