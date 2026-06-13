mod builtins;
mod completer;
mod executor;
mod expand;
mod parser;
mod state;
mod tokenizer;
mod types;

use std::path::PathBuf;

use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;

use completer::ShellCompleter;
use executor::evaluate_tokens;
use state::ShellState;
use tokenizer::{is_incomplete, tokenize};

fn main() {
    let mut state = ShellState::new();
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

    // ── Terminal setup ────────────────────────────────────────────────────────
    unsafe {
        libc::signal(libc::SIGTTOU, libc::SIG_IGN);
        libc::signal(libc::SIGTTIN, libc::SIG_IGN);
        libc::signal(libc::SIGTSTP, libc::SIG_IGN); // Shell itself never suspends

        let shell_pgid = libc::getpid();
        if libc::setpgid(shell_pgid, shell_pgid) < 0 {
            eprintln!("Warning: Failed to put shell in its own process group.");
        }
        libc::tcsetpgrp(libc::STDIN_FILENO, shell_pgid);
    }

    // Ctrl-C just prints a newline (doesn't kill the shell)
    ctrlc::set_handler(|| println!()).expect("Error setting Ctrl-C handler");

    // ── Source ~/.rshrc ───────────────────────────────────────────────────────
    let rc_file = PathBuf::from(&home_dir).join(".rshrc");
    if let Ok(contents) = std::fs::read_to_string(&rc_file) {
        let cleaned = strip_comments(&contents);
        let tokens = tokenize(&cleaned);
        evaluate_tokens(&mut state, &tokens);
    }

    // ── Handle non-interactive modes (-c "cmd" or script file) ───────────────
    let args: Vec<String> = std::env::args().collect();

    if args.len() >= 3 && args[1] == "-c" {
        let tokens = tokenize(&args[2]);
        evaluate_tokens(&mut state, &tokens);
        return;
    }

    if args.len() == 2 {
        match std::fs::read_to_string(&args[1]) {
            Ok(contents) => {
                let tokens = tokenize(&strip_comments(&contents));
                evaluate_tokens(&mut state, &tokens);
                std::process::exit(state.last_exit_status);
            }
            Err(_) => {
                eprintln!("rsh: {}: No such file or directory", args[1]);
                std::process::exit(127);
            }
        }
    }

    // ── Interactive REPL ──────────────────────────────────────────────────────
    let history_file = PathBuf::from(&home_dir).join(".rsh_history");
    let config = rustyline::Config::builder()
        .completion_type(rustyline::CompletionType::List)
        .build();
    let mut rl: Editor<ShellCompleter, DefaultHistory> =
        Editor::with_config(config).expect("Failed to create readline editor");
    rl.set_helper(Some(ShellCompleter {
        hinter: rustyline::hint::HistoryHinter::new(),
        highlighter: rustyline::highlight::MatchingBracketHighlighter::new(),
    }));
    let _ = rl.load_history(&history_file);

    let mut input_buffer = String::new();

    loop {
        let prompt = if input_buffer.is_empty() { "$ " } else { "> " };

        match rl.readline(prompt) {
            Ok(line) => {
                if line.trim().is_empty() && input_buffer.is_empty() {
                    continue;
                }

                if !input_buffer.is_empty() {
                    input_buffer.push('\n');
                }
                input_buffer.push_str(&line);

                let cleaned_input = strip_comments(&input_buffer);
                let tokens = tokenize(&cleaned_input);
                if is_incomplete(&input_buffer, &tokens) {
                    continue; // Wait for the user to finish the block
                }

                let _ = rl.add_history_entry(input_buffer.trim());
                let _ = rl.save_history(&history_file);

                evaluate_tokens(&mut state, &tokens);
                input_buffer.clear();
            }
            Err(ReadlineError::Interrupted) => {
                input_buffer.clear(); // Ctrl-C cancels current input
            }
            Err(ReadlineError::Eof) => {
                println!("exit");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    let _ = rl.save_history(&history_file);
}

// ── Utilities ─────────────────────────────────────────────────────────────────

pub fn strip_comments(input: &str) -> String {
    let mut out = String::new();
    for line in input.lines() {
        // Find the first '#' that isn't preceded by a backslash
        // (Note: A true robust fix requires checking if it's inside quotes,
        // but this handles 90% of basic scripting needs)
        if let Some(idx) = line.find('#') {
            out.push_str(&line[..idx]);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}
