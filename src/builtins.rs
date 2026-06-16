use crate::expand::{find_in_path, is_tty};
use crate::state::{Job, JobStatus, ShellState};
use crate::strip_comments;
use crate::types::Builtin;
use std::io::Write;

// ANSI color codes — only used when stdout is a real TTY
const BOLD_GREEN: &str = "\x1b[1;32m";
const BOLD_YELLOW: &str = "\x1b[1;33m";
const RESET: &str = "\x1b[0m";

/// Execute a builtin, writing stdout to `output` and stderr to `err_output`.
/// Returns `Some(bool)` on success/failure, `None` if the builtin is not
/// handled here (e.g. Exit, which never returns).
pub fn run_builtin<W: Write, E: Write>(
    state: &mut ShellState,
    builtin: Builtin,
    output: &mut W,
    err_output: &mut E,
) -> bool {
    match builtin {
        Builtin::Exit(code) => std::process::exit(code),

        Builtin::Echo(args) => {
            writeln!(output, "{}", args.join(" ")).unwrap();
            true
        }

        Builtin::Pwd => {
            writeln!(output, "{}", std::env::current_dir().unwrap().display()).unwrap();
            true
        }

        Builtin::Cd(target_opt) => {
            let target = match target_opt {
                Some(dir) => dir,
                None => std::env::var("HOME").unwrap_or_else(|_| "/".to_string()),
            };

            if let Err(_) = std::env::set_current_dir(&target) {
                let _ = writeln!(err_output, "rsh: cd: {}: No such file or directory", target);
                false
            } else {
                true
            }
        }
        Builtin::Test(args) => evaluate_test(&args),

        Builtin::Export(args) => {
            for arg in args {
                if let Some((k, v)) = arg.split_once('=') {
                    std::env::set_var(k, v);
                }
            }
            true
        }

        Builtin::Unset(args) => {
            for arg in args {
                if arg == "-f" {
                    continue;
                } // Ignore flags like -f
                std::env::remove_var(&arg);
                state.functions.remove(&arg); // Python scripts sometimes unset functions
            }
            true
        }

        Builtin::Hash(_) => {
            // Python's activate script calls `hash -r` to clear the binary cache.
            // Since we don't cache binary paths yet, we just return success!
            true
        }

        Builtin::Type(commands) => {
            let color = is_tty(libc::STDOUT_FILENO);
            for cmd in commands {
                match cmd.as_str() {
                    "echo" | "exit" | "type" | "cd" | "pwd" | "export" | "alias" | "jobs"
                    | "fg" | "bg" | "source" => {
                        if color {
                            writeln!(output, "{} is {}shell builtin{}", cmd, BOLD_GREEN, RESET)
                                .unwrap();
                        } else {
                            writeln!(output, "{} is a shell builtin", cmd).unwrap();
                        }
                    }
                    _ => match find_in_path(&cmd) {
                        Some(p) => {
                            if color {
                                writeln!(
                                    output,
                                    "{} is {}{}{}",
                                    cmd,
                                    BOLD_YELLOW,
                                    p.display(),
                                    RESET
                                )
                                .unwrap();
                            } else {
                                writeln!(output, "{} is {}", cmd, p.display()).unwrap();
                            }
                        }
                        None => writeln!(output, "{}: not found", cmd).unwrap(),
                    },
                }
            }
            true
        }

        Builtin::Alias(args) => {
            if args.is_empty() {
                for (k, v) in &state.aliases {
                    writeln!(output, "alias {}='{}'", k, v).unwrap();
                }
            } else {
                for arg in args {
                    if let Some((key, value)) = arg.split_once('=') {
                        let clean = value.trim_matches(|c| c == '\'' || c == '"');
                        state.aliases.insert(key.to_string(), clean.to_string());
                    }
                }
            }
            true
        }

        Builtin::Jobs => {
            let color = is_tty(libc::STDOUT_FILENO);
            state.jobs.retain(|job| {
                let mut status = 0;
                unsafe { libc::waitpid(job.pgid, &mut status, libc::WNOHANG) == 0 }
            });
            for job in &state.jobs {
                let status_str = match job.status {
                    JobStatus::Running => {
                        if color {
                            format!("{}Running{}", BOLD_GREEN, RESET)
                        } else {
                            "Running".to_string()
                        }
                    }
                    JobStatus::Stopped => {
                        if color {
                            format!("{}Stopped{}", BOLD_YELLOW, RESET)
                        } else {
                            "Stopped".to_string()
                        }
                    }
                };
                writeln!(output, "[{}]  {}    {}", job.id, status_str, job.command).unwrap();
            }
            true
        }

        Builtin::Fg(target_id) => {
            let id = target_id.unwrap_or(1);
            if let Some(pos) = state.jobs.iter().position(|j| j.id == id) {
                let job = state.jobs.remove(pos);
                println!("{}", job.command);
                unsafe {
                    libc::kill(-job.pgid, libc::SIGCONT);
                    libc::tcsetpgrp(libc::STDIN_FILENO, job.pgid);
                    let mut status = 0;
                    libc::waitpid(job.pgid, &mut status, libc::WUNTRACED);
                    libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpid());

                    if libc::WIFSTOPPED(status) {
                        println!("\n[{}] + Stopped          {}", id, job.command);
                        state.jobs.push(Job {
                            status: JobStatus::Stopped,
                            ..job
                        });
                    }
                }
            } else {
                writeln!(err_output, "rsh: fg: No such job: {}", id).unwrap();
            }
            true
        }
        Builtin::ReadJson(path) => match std::fs::File::open(&path) {
            Ok(file) => match serde_json::from_reader::<_, serde_json::Value>(file) {
                Ok(serde_json::Value::Object(map)) => {
                    for (key, value) in map {
                        let val_str = match value {
                            serde_json::Value::String(s) => s,
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            _ => value.to_string(),
                        };
                        state.json_vars.insert(key, val_str);
                    }
                    true
                }
                _ => {
                    let _ = writeln!(
                        err_output,
                        "rsh: readjson: {} is not a valid JSON object",
                        path
                    );
                    false
                }
            },
            Err(e) => {
                let _ = writeln!(err_output, "rsh: readjson: {}: {}", path, e);
                false
            }
        },

        Builtin::Bg(target_id) => {
            let id = target_id.unwrap_or(1);
            if let Some(job) = state.jobs.iter_mut().find(|j| j.id == id) {
                if job.status == JobStatus::Stopped {
                    unsafe {
                        libc::kill(-job.pgid, libc::SIGCONT);
                    }
                    job.status = JobStatus::Running;
                    println!("[{}] {} &", job.id, job.command);
                }
            } else {
                writeln!(err_output, "rsh: bg: No such job: {}", id).unwrap();
            }
            true
        }

        Builtin::RegexMatch(text, pattern) => match regex::Regex::new(&pattern) {
            Ok(re) => re.is_match(&text),
            Err(e) => {
                eprintln!("rsh: regex syntax error: {}", e);
                false
            }
        },

        Builtin::Source(path) => match std::fs::read_to_string(&path) {
            Ok(contents) => {
                let cleaned = strip_comments(&contents);
                let tokens = crate::tokenizer::tokenize(&cleaned);
                crate::executor::evaluate_tokens(state, &tokens)
            }
            Err(e) => {
                eprintln!("rsh: source: {}: {}", path, e);
                false
            }
        },
    }
}
fn evaluate_test_simple(args: &[String]) -> bool {
    match args.len() {
        0 => false,
        1 => !args[0].is_empty(), // Single string is true if not empty
        2 => {
            // Unary operators (e.g., -f file.txt)
            let op = args[0].as_str();
            let val = args[1].as_str();
            match op {
                // String tests
                "-z" => val.is_empty(),
                "-n" => !val.is_empty(),
                // File tests
                "-e" => std::path::Path::new(val).exists(),
                "-f" => std::path::Path::new(val).is_file(),
                "-d" => std::path::Path::new(val).is_dir(),
                "-L" => std::fs::symlink_metadata(val)
                    .map(|m| m.is_symlink())
                    .unwrap_or(false),
                "-x" => {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::metadata(val)
                        .map(|m| m.permissions().mode() & 0o111 != 0)
                        .unwrap_or(false)
                }
                // Type checks
                "-isint" => val.parse::<i64>().is_ok(),
                "-isnum" => val.parse::<f64>().is_ok(),
                _ => {
                    eprintln!("rsh: test: {}: unary operator expected", op);
                    false
                }
            }
        }
        3 => {
            // Binary operators (e.g., $A -eq $B, or $A == $B)
            let left = args[0].as_str();
            let op = args[1].as_str();
            let right = args[2].as_str();

            match op {
                // String comparisons
                "=" | "==" => left == right,
                "!=" => left != right,
                // Integer comparisons
                "-eq" | "-ne" | "-gt" | "-ge" | "-lt" | "-le" => {
                    let l: i64 = left.parse().unwrap_or(0);
                    let r: i64 = right.parse().unwrap_or(0);
                    match op {
                        "-eq" => l == r,
                        "-ne" => l != r,
                        "-gt" => l > r,
                        "-ge" => l >= r,
                        "-lt" => l < r,
                        "-le" => l <= r,
                        _ => unreachable!(),
                    }
                }
                // String inclusion
                "-contains" => left.contains(right),
                "-starts" => left.starts_with(right),
                "-ends" => left.ends_with(right),
                // Native Grep (File contains string)
                "-fcontains" => {
                    if let Ok(contents) = std::fs::read_to_string(left) {
                        contents.contains(right)
                    } else {
                        false
                    }
                }

                _ => {
                    eprintln!("rsh: test: {}: binary operator expected", op);
                    false
                }
            }
        }
        _ => {
            eprintln!("rsh: test: too many arguments");
            false
        }
    }
}
fn evaluate_test(args: &[String]) -> bool {
    let mut eval_args = args;
    let mut invert = false;

    if !eval_args.is_empty() && eval_args[0] == "!" {
        invert = true;
        eval_args = &eval_args[1..];
    }

    if let Some(pos) = eval_args.iter().position(|a| a == "-o") {
        let left = evaluate_test(&eval_args[..pos].to_vec());
        let right = evaluate_test(&eval_args[pos + 1..].to_vec());
        let res = left || right;
        return if invert { !res } else { res };
    }

    if let Some(pos) = eval_args.iter().position(|a| a == "-a") {
        let left = evaluate_test(&eval_args[..pos].to_vec());
        let right = evaluate_test(&eval_args[pos + 1..].to_vec());
        let res = left && right;
        return if invert { !res } else { res };
    }

    let res = evaluate_test_simple(eval_args);
    if invert {
        !res
    } else {
        res
    }
}
