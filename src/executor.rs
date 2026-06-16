use crate::builtins::run_builtin;
use crate::expand::{expand_word, find_in_path};
use crate::parser::{parse_ast, parse_pipeline_from_tokens};
use crate::state::{Job, JobStatus, ShellState};
use crate::types::{ASTNode, Builtin, Command};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::process::CommandExt;

// ── Top-level entry point ─────────────────────────────────────────────────────

pub fn evaluate_tokens(state: &mut ShellState, tokens: &[String]) -> bool {
    if let Some(ast) = parse_ast(state, tokens) {
        evaluate_ast(state, &ast)
    } else {
        eprintln!("rsh: syntax error");
        state.last_exit_status = 258;
        false
    }
}

// ── AST evaluator ─────────────────────────────────────────────────────────────

pub fn evaluate_ast(state: &mut ShellState, node: &ASTNode) -> bool {
    match node {
        ASTNode::Block(nodes) => {
            let mut last = true;
            for n in nodes {
                last = evaluate_ast(state, n);
            }
            last
        }

        ASTNode::FunctionDef { name, body } => {
            state.functions.insert(name.clone(), *body.clone());
            true
        }

        ASTNode::LogicalAnd(left, right) => evaluate_ast(state, left) && evaluate_ast(state, right),
        ASTNode::LogicalOr(left, right) => {
            let ok = evaluate_ast(state, left);
            if ok {
                true
            } else {
                evaluate_ast(state, right)
            }
        }

        ASTNode::If {
            condition,
            then_branch,
            else_branch,
        } => {
            if evaluate_ast(state, condition) {
                evaluate_ast(state, then_branch)
            } else if let Some(else_node) = else_branch {
                evaluate_ast(state, else_node)
            } else {
                state.last_exit_status = 0;
                true
            }
        }

        ASTNode::While { condition, body } => {
            let mut last = true;
            while evaluate_ast(state, condition) {
                last = evaluate_ast(state, body);
            }
            last
        }

        ASTNode::For {
            var_name,
            items,
            body,
        } => {
            let mut last = true;
            for item in items {
                std::env::set_var(var_name, expand_word(state, item));
                last = evaluate_ast(state, body);
            }
            last
        }

        ASTNode::Pipeline(tokens, background) => {
            if tokens.is_empty() {
                return true;
            }

            if tokens[0] == "time" {
                if tokens.len() == 1 {
                    return true; // Nothing to time
                }

                let start_time = std::time::Instant::now();
                let mut usage_start = unsafe { std::mem::zeroed::<libc::rusage>() };
                unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage_start) };

                let inner_node = ASTNode::Pipeline(tokens[1..].to_vec(), *background);
                let result = evaluate_ast(state, &inner_node);

                let mut usage_end = unsafe { std::mem::zeroed::<libc::rusage>() };
                unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage_end) };

                let real = start_time.elapsed().as_secs_f64();

                let to_sec =
                    |tv: libc::timeval| tv.tv_sec as f64 + (tv.tv_usec as f64 / 1_000_000.0);
                let user = to_sec(usage_end.ru_utime) - to_sec(usage_start.ru_utime);
                let sys = to_sec(usage_end.ru_stime) - to_sec(usage_start.ru_stime);

                eprintln!("\nreal\t{:.3}s\nuser\t{:.3}s\nsys\t{:.3}s", real, user, sys);
                return result;
            }
            // ──────────────────────────────────────────

            if let Some(func_body) = state.functions.get(&tokens[0]).cloned() {
                // Inject $1, $2, … into environment
                let mut saved: Vec<Option<String>> = Vec::new();
                for (i, token) in tokens[1..].iter().enumerate() {
                    saved.push(std::env::var((i + 1).to_string()).ok());
                    std::env::set_var((i + 1).to_string(), expand_word(state, token));
                }

                let status = evaluate_ast(state, &func_body);

                // Restore previous values
                for (i, old) in saved.into_iter().enumerate() {
                    let key = (i + 1).to_string();
                    match old {
                        Some(v) => std::env::set_var(&key, v),
                        None => std::env::remove_var(&key),
                    }
                }
                return status;
            }

            let commands = parse_pipeline_from_tokens(state, tokens);
            if commands.len() == 1 {
                execute_single(state, &commands[0], *background)
            } else {
                execute_pipeline(state, &commands, *background)
            }
        }
    }
}

// ── Single-command execution ──────────────────────────────────────────────────

pub fn execute_single(state: &mut ShellState, cmd: &Command, background: bool) -> bool {
    let mut stdout_sink: Box<dyn Write> = open_stdout(&cmd.stdout_file, cmd.append_stdout);
    let mut stderr_sink: Box<dyn Write> = open_stderr(&cmd.stderr_file, cmd.append_stderr);

    // Alias expansion
    let (cmd_name, cmd_args) = expand_alias(state, &cmd.command, &cmd.args);
    // If the command is just `VAR=value` with no arguments:
    if cmd_args.is_empty() && cmd_name.contains('=') {
        if let Some((name, value)) = cmd_name.split_once('=') {
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                std::env::set_var(name, value);
                state.last_exit_status = 0;
                return true;
            }
        }
    }

    if let Some(builtin) = Builtin::parse(&cmd_name, &cmd_args) {
        let ok = run_builtin(state, builtin, &mut stdout_sink, &mut stderr_sink);
        state.last_exit_status = if ok { 0 } else { 1 };
        return ok;
    }

    // External command
    let Some(full_path) = find_in_path(&cmd_name) else {
        println!("{}: command not found", cmd_name);
        state.last_exit_status = 127;
        return false;
    };

    let mut child = std::process::Command::new(full_path);
    child.args(&cmd_args);

    // Input redirection
    if let Some(in_file) = &cmd.stdin_file {
        match std::fs::File::open(in_file) {
            Ok(f) => {
                child.stdin(std::process::Stdio::from(f));
            }
            Err(_) => {
                eprintln!("{}: No such file or directory", in_file);
                state.last_exit_status = 1;
                return false;
            }
        }
    } else if cmd.heredoc_content.is_some() {
        child.stdin(std::process::Stdio::piped());
    }

    attach_output_redirects(
        &mut child,
        &cmd.stdout_file,
        cmd.append_stdout,
        &cmd.stderr_file,
        cmd.append_stderr,
    );

    setup_child_signals(&mut child, cmd.merge_stderr);

    match child.spawn() {
        Err(e) => {
            eprintln!("{}: {}", cmd_name, e);
            state.last_exit_status = 126;
            false
        }
        Ok(mut spawned) => {
            let pid = spawned.id() as i32;
            let pgid = pid;
            if let Some(content) = &cmd.heredoc_content {
                if let Some(mut stdin) = spawned.stdin.take() {
                    // Write the string and drop the pipe so the child sees EOF
                    let _ = stdin.write_all(content.as_bytes());
                }
            }

            if background {
                let job_id = state.job_id_counter;
                state.jobs.push(Job {
                    id: job_id,
                    pgid,
                    command: format!("{} {}", cmd_name, cmd_args.join(" ")),
                    status: JobStatus::Running,
                });
                println!("[{}] {}", job_id, pid);
                state.job_id_counter += 1;
                state.last_exit_status = 0;
                true
            } else {
                wait_foreground(state, spawned, pgid, &cmd_name, &cmd_args)
            }
        }
    }
}

// ── Pipeline execution ────────────────────────────────────────────────────────

pub fn execute_pipeline(state: &mut ShellState, pipeline: &[Command], background: bool) -> bool {
    let mut previous_stdout: Option<std::process::ChildStdout> = None;
    let mut builtin_buffer: Option<Vec<u8>> = None;
    let mut final_success = true;

    for (i, cmd) in pipeline.iter().enumerate() {
        let is_last = i == pipeline.len() - 1;
        let (cmd_name, cmd_args) = expand_alias(state, &cmd.command, &cmd.args);

        if let Some(builtin) = Builtin::parse(&cmd_name, &cmd_args) {
            let mut buf = Vec::new();
            let mut err = std::io::stderr();
            match builtin {
                Builtin::Echo(_)
                | Builtin::Pwd
                | Builtin::Type(_)
                | Builtin::Alias(_)
                | Builtin::Jobs
                | Builtin::RegexMatch(..) => {
                    run_builtin(state, builtin, &mut buf, &mut err);
                }
                _ => {
                    run_builtin(state, builtin, &mut buf, &mut err);
                }
            }

            if is_last {
                std::io::stdout().write_all(&buf).unwrap();
                state.last_exit_status = 0;
                final_success = true;
            } else {
                builtin_buffer = Some(buf);
            }
        } else {
            let Some(full_path) = find_in_path(&cmd_name) else {
                println!("{}: command not found", cmd_name);
                state.last_exit_status = 127;
                return false;
            };

            let mut child = std::process::Command::new(full_path);
            child.args(&cmd_args);

            // Pipe stdin from previous stage
            if let Some(out) = previous_stdout.take() {
                child.stdin(std::process::Stdio::from(out));
            } else if builtin_buffer.is_some() {
                child.stdin(std::process::Stdio::piped());
            } else if cmd.heredoc_content.is_some() {
                child.stdin(std::process::Stdio::piped());
            }

            if !is_last {
                child.stdout(std::process::Stdio::piped());
            }

            // Stderr redirect on individual commands
            if let Some(err_file) = &cmd.stderr_file {
                let file = open_file(err_file, cmd.append_stderr);
                child.stderr(std::process::Stdio::from(file));
            }

            setup_child_signals(&mut child, cmd.merge_stderr);
            let mut spawned = child.spawn().expect("failed to spawn");

            if let Some(buf) = builtin_buffer.take() {
                if let Some(mut stdin) = spawned.stdin.take() {
                    stdin.write_all(&buf).unwrap();
                }
            } else if let Some(content) = &cmd.heredoc_content {
                if let Some(mut stdin) = spawned.stdin.take() {
                    let _ = stdin.write_all(content.as_bytes());
                }
            }
            if !is_last {
                previous_stdout = spawned.stdout.take();
            } else if background {
                println!("[1] {}", spawned.id());
                state.last_exit_status = 0;
                final_success = true;
            } else {
                let status = spawned.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
                state.last_exit_status = status;
                final_success = status == 0;
            }
        }
    }

    final_success
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn expand_alias(state: &ShellState, cmd: &str, args: &[String]) -> (String, Vec<String>) {
    if let Some(expanded) = state.aliases.get(cmd) {
        let mut parts: Vec<String> = expanded.split_whitespace().map(String::from).collect();
        if !parts.is_empty() {
            let new_cmd = parts.remove(0);
            parts.extend_from_slice(args);
            return (new_cmd, parts);
        }
    }
    (cmd.to_string(), args.to_vec())
}

fn open_stdout(path: &Option<String>, append: bool) -> Box<dyn Write> {
    match path {
        Some(file) => Box::new(open_file(file, append)),
        None => Box::new(std::io::stdout()),
    }
}

fn open_stderr(path: &Option<String>, append: bool) -> Box<dyn Write> {
    match path {
        Some(file) => Box::new(open_file(file, append)),
        None => Box::new(std::io::stderr()),
    }
}

fn open_file(path: &str, append: bool) -> File {
    if append {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap()
    } else {
        File::create(path).unwrap()
    }
}

fn attach_output_redirects(
    child: &mut std::process::Command,
    stdout_file: &Option<String>,
    append_stdout: bool,
    stderr_file: &Option<String>,
    append_stderr: bool,
) {
    if let Some(f) = stdout_file {
        child.stdout(std::process::Stdio::from(open_file(f, append_stdout)));
    }
    if let Some(f) = stderr_file {
        child.stderr(std::process::Stdio::from(open_file(f, append_stderr)));
    }
}

fn setup_child_signals(child: &mut std::process::Command, merge_stderr: bool) {
    unsafe {
        child.pre_exec(move || {
            libc::setpgid(0, 0);
            libc::signal(libc::SIGINT, libc::SIG_DFL);
            libc::signal(libc::SIGQUIT, libc::SIG_DFL);
            libc::signal(libc::SIGTSTP, libc::SIG_DFL);
            libc::signal(libc::SIGTTIN, libc::SIG_DFL);
            libc::signal(libc::SIGTTOU, libc::SIG_DFL);

            if merge_stderr {
                if libc::dup2(libc::STDOUT_FILENO, libc::STDERR_FILENO) < 0 {
                    eprintln!("rsh: failed to merge stderr into stdout");
                    libc::_exit(1);
                }
            }

            Ok(())
        });
    }
}

fn wait_foreground(
    state: &mut ShellState,
    spawned: std::process::Child,
    pgid: i32,
    cmd_name: &str,
    cmd_args: &[String],
) -> bool {
    unsafe {
        libc::tcsetpgrp(libc::STDIN_FILENO, pgid);
    }

    let mut raw_status: libc::c_int = 0;
    unsafe {
        libc::waitpid(pgid, &mut raw_status, libc::WUNTRACED);
    }
    unsafe {
        libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpid());
    }

    drop(spawned);

    if libc::WIFSTOPPED(raw_status) {
        let job_id = state.job_id_counter;
        state.jobs.push(Job {
            id: job_id,
            pgid,
            command: format!("{} {}", cmd_name, cmd_args.join(" ")),
            status: JobStatus::Stopped,
        });
        println!("\n[{}] + Stopped          {}", job_id, cmd_name);
        state.job_id_counter += 1;
        state.last_exit_status = 148;
        true
    } else {
        state.last_exit_status = if libc::WIFEXITED(raw_status) {
            libc::WEXITSTATUS(raw_status)
        } else {
            1
        };
        state.last_exit_status == 0
    }
}
