use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub enum JobStatus {
    Running,
    Stopped,
}

pub struct Job {
    pub id: usize,
    pub pgid: i32,
    pub command: String,
    pub status: JobStatus,
}

struct ShellState {
    aliases: HashMap<String, String>,
    jobs: Vec<Job>,
    job_id_counter: usize,
    last_exit_status: i32,
}

impl ShellState {
    fn new() -> Self {
        ShellState {
            aliases: HashMap::new(),
            jobs: Vec::new(),
            job_id_counter: 1,
            last_exit_status: 0,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Operator {
    And,
    Or,
    Async,
    None,
}

struct LogicalGroup {
    pipeline: Vec<Command>,
    next_op: Operator,
}

enum Builtin {
    Exit(i32),
    Echo(Vec<String>),
    Type(Vec<String>),
    Pwd,
    Cd(String),
    Export(String, String),
    Alias(Vec<String>),
    Jobs,
    Fg(Option<usize>),
    Bg(Option<usize>),
}

impl Builtin {
    fn parse(command: &str, args: &[String]) -> Option<Builtin> {
        match command {
            "exit" => {
                let code = args
                    .first()
                    .and_then(|c| c.parse::<i32>().ok())
                    .unwrap_or(0);
                Some(Builtin::Exit(code))
            }
            "echo" => {
                let echo_args = args.iter().map(|s| s.to_string()).collect();
                Some(Builtin::Echo(echo_args))
            }
            "type" => {
                let type_args = args.iter().map(|s| s.to_string()).collect();
                Some(Builtin::Type(type_args))
            }
            "cd" => {
                let path = args.first().map(|s| s.clone()).unwrap_or_default();
                Some(Builtin::Cd(path))
            }
            "pwd" => Some(Builtin::Pwd),
            "export" => {
                if let Some(arg) = args.first() {
                    if let Some((key, value)) = arg.split_once('=') {
                        return Some(Builtin::Export(key.to_string(), value.to_string()));
                    }
                }
                None
            }
            "alias" => {
                let alias_args = args.iter().map(|s| s.to_string()).collect();
                Some(Builtin::Alias(alias_args))
            }
            "jobs" => Some(Builtin::Jobs),
            "fg" => {
                let id = args.first().and_then(|s| s.parse::<usize>().ok());
                Some(Builtin::Fg(id))
            }
            "bg" => {
                let id = args.first().and_then(|s| s.parse::<usize>().ok());
                Some(Builtin::Bg(id))
            }
            _ => None,
        }
    }
}

struct Command {
    command: String,
    args: Vec<String>,
    stdin_file: Option<String>,
    stdout_file: Option<String>,
    append_stdout: bool,
    stderr_file: Option<String>,
    append_stderr: bool,
}

impl Command {
    fn from_tokens(state: &ShellState, tokens: Vec<String>) -> Command {
        if tokens.is_empty() {
            return Command {
                command: String::new(),
                args: Vec::new(),
                stdin_file: None,
                stdout_file: None,
                append_stdout: false,
                stderr_file: None,
                append_stderr: false,
            };
        }

        let mut args = Vec::new();
        let mut stdin_file = None;
        let mut stdout_file = None;
        let mut append_stdout = false;
        let mut stderr_file = None;
        let mut append_stderr = false;

        let mut i = 0;
        while i < tokens.len() {
            match tokens[i].as_str() {
                "<" => {
                    if i + 1 < tokens.len() {
                        stdin_file = Some(expand_word(state, &tokens[i + 1]));
                        i += 1;
                    }
                }
                ">" | "1>" => {
                    if i + 1 < tokens.len() {
                        stdout_file = Some(expand_word(state, &tokens[i + 1]));
                        append_stdout = false;
                        i += 1;
                    }
                }
                ">>" | "1>>" => {
                    if i + 1 < tokens.len() {
                        stdout_file = Some(expand_word(state, &tokens[i + 1]));
                        append_stdout = true;
                        i += 1;
                    }
                }

                "2>" => {
                    if i + 1 < tokens.len() {
                        stderr_file = Some(expand_word(state, &tokens[i + 1]));
                        append_stderr = false;
                        i += 1;
                    }
                }
                "2>>" => {
                    if i + 1 < tokens.len() {
                        stderr_file = Some(expand_word(state, &tokens[i + 1]));
                        append_stderr = true;
                        i += 1;
                    }
                }
                _ => {
                    let expanded = expand_word(state, &tokens[i]);
                    let mut globbed = expand_glob(&expanded);
                    args.append(&mut globbed);
                }
            }
            i += 1;
        }

        let command = if !args.is_empty() {
            args.remove(0)
        } else {
            String::new()
        };
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());

        for arg in args.iter_mut() {
            if arg == "~" {
                *arg = home_dir.clone();
            } else if arg.starts_with("~/") {
                *arg = arg.replacen('~', &home_dir, 1);
            }
        }

        if let Some(ref mut file) = stdin_file {
            if file == "~" {
                *file = home_dir.clone();
            } else if file.starts_with("~/") {
                *file = file.replacen('~', &home_dir, 1);
            }
        }

        if let Some(ref mut file) = stdout_file {
            if file == "~" {
                *file = home_dir.clone();
            } else if file.starts_with("~/") {
                *file = file.replacen('~', &home_dir, 1);
            }
        }
        if let Some(ref mut file) = stderr_file {
            if file == "~" {
                *file = home_dir.clone();
            } else if file.starts_with("~/") {
                *file = file.replacen('~', &home_dir, 1);
            }
        }

        Command {
            command,
            args,
            stdin_file,
            stdout_file,
            append_stdout,
            stderr_file,
            append_stderr,
        }
    }
}

fn match_pattern(pattern: &str, text: &str) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    if pattern.starts_with('*') {
        match_pattern(&pattern[1..], text)
            || (!text.is_empty() && match_pattern(pattern, &text[1..]))
    } else {
        let p_char = pattern.chars().next().unwrap();
        if text.starts_with(p_char) {
            match_pattern(&pattern[p_char.len_utf8()..], &text[p_char.len_utf8()..])
        } else {
            false
        }
    }
}

fn expand_glob(word: &str) -> Vec<String> {
    if !word.contains('*') {
        return vec![word.to_string()];
    }

    let mut matches = Vec::new();
    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.starts_with('.') && !word.starts_with('.') {
                    continue;
                }
                if match_pattern(word, &name) {
                    matches.push(name);
                }
            }
        }
    }

    if matches.is_empty() {
        vec![word.to_string()]
    } else {
        matches.sort();
        matches
    }
}

pub fn expand_word(state: &ShellState, input: &str) -> String {
    let mut result = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if escaped {
            result.push(c);
            escaped = false;
            i += 1;
            continue;
        }

        match c {
            '\\' => {
                if in_single {
                    result.push(c); // Backslashes are literal in single quotes
                } else {
                    escaped = true; // Skip this slash, escape the next char
                }
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '$' if !in_single => {
                // --- NEW: ARITHMETIC EXPANSION $((...)) ---
                if i + 2 < chars.len() && chars[i + 1] == '(' && chars[i + 2] == '(' {
                    i += 3; // Skip '$(('
                    let mut math_expr = String::new();

                    // Keep reading until we hit the double closing parenthesis '))'
                    while i + 1 < chars.len() && !(chars[i] == ')' && chars[i + 1] == ')') {
                        math_expr.push(chars[i]);
                        i += 1;
                    }
                    i += 2; // Skip '))'

                    // Evaluate and append!
                    match eval_math(&math_expr) {
                        Ok(num) => result.push_str(&num.to_string()),
                        Err(e) => eprintln!("rsh: math error: {}", e),
                    }
                    continue;
                }

                // 1. COMMAND SUBSTITUTION $(...)
                if i + 1 < chars.len() && chars[i + 1] == '(' {
                    // ... rest of your code ...
                    i += 2; // Skip '$' and '('
                    let mut sub_cmd = String::new();
                    let mut paren_count = 1;

                    while i < chars.len() && paren_count > 0 {
                        if chars[i] == '(' {
                            paren_count += 1;
                        } else if chars[i] == ')' {
                            paren_count -= 1;
                        }

                        if paren_count > 0 {
                            sub_cmd.push(chars[i]);
                        }
                        i += 1;
                    }

                    if let Ok(exe) = std::env::current_exe() {
                        if let Ok(output) = std::process::Command::new(exe)
                            .arg("-c")
                            .arg(&sub_cmd)
                            .output()
                        {
                            let out_str = String::from_utf8_lossy(&output.stdout);
                            result.push_str(out_str.trim_end_matches('\n'));
                        }
                    }
                    continue;
                }

                // 2. EXIT STATUS $?
                if i + 1 < chars.len() && chars[i + 1] == '?' {
                    result.push_str(&state.last_exit_status.to_string());
                    i += 2;
                    continue;
                }

                // 3. BRACE EXPANSION ${VAR:-default}
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    i += 2; // Skip '${'
                    let mut inside_brace = String::new();
                    while i < chars.len() && chars[i] != '}' {
                        inside_brace.push(chars[i]);
                        i += 1;
                    }
                    if i < chars.len() && chars[i] == '}' {
                        i += 1; // Skip '}'
                    }

                    // Parse the default value fallback ":-"
                    if let Some((var_name, default_val)) = inside_brace.split_once(":-") {
                        if let Ok(val) = std::env::var(var_name) {
                            if !val.is_empty() {
                                result.push_str(&val);
                            } else {
                                result.push_str(default_val);
                            }
                        } else {
                            result.push_str(default_val);
                        }
                    } else {
                        // Standard ${VAR} with no default
                        if let Ok(val) = std::env::var(&inside_brace) {
                            result.push_str(&val);
                        }
                    }
                    continue;
                }

                // 4. STANDARD VARIABLE EXPANSION $VAR
                let mut var_name = String::new();
                i += 1; // Skip '$'
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    var_name.push(chars[i]);
                    i += 1;
                }

                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                }
                continue;
            }
            _ => {
                result.push(c);
            }
        }
        i += 1;
    }

    result
}

fn parse_logic(state: &ShellState, tokens: &[String]) -> Vec<LogicalGroup> {
    let mut groups = Vec::new();
    let mut current_tokens = Vec::new();

    for token in tokens {
        match token.as_str() {
            "&&" => {
                let pipeline = parse_pipeline_from_tokens(state, &current_tokens);
                groups.push(LogicalGroup {
                    pipeline,
                    next_op: Operator::And,
                });
                current_tokens.clear();
            }
            "||" => {
                let pipeline = parse_pipeline_from_tokens(state, &current_tokens);
                groups.push(LogicalGroup {
                    pipeline,
                    next_op: Operator::Or,
                });
                current_tokens.clear();
            }
            "&" => {
                let pipeline = parse_pipeline_from_tokens(state, &current_tokens);
                groups.push(LogicalGroup {
                    pipeline,
                    next_op: Operator::Async,
                });
                current_tokens.clear();
            }
            _ => current_tokens.push(token.clone()),
        }
    }

    if !current_tokens.is_empty() {
        let pipeline = parse_pipeline_from_tokens(state, &current_tokens);
        groups.push(LogicalGroup {
            pipeline,
            next_op: Operator::None,
        });
    }

    groups
}

fn parse_pipeline_from_tokens(state: &ShellState, tokens: &[String]) -> Vec<Command> {
    let mut commands = Vec::new();
    let mut current_tokens = Vec::new();

    for token in tokens {
        if token == "|" {
            commands.push(Command::from_tokens(state, current_tokens.clone()));
            current_tokens.clear();
        } else {
            current_tokens.push(token.clone());
        }
    }
    commands.push(Command::from_tokens(state, current_tokens));
    commands
}

fn evaluate_tokens(state: &mut ShellState, tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return true;
    }

    if tokens[0] == "for" {
        let in_pos = tokens.iter().position(|t| t == "in");
        let do_pos = tokens.iter().position(|t| t == "do");
        let done_pos = tokens.iter().rposition(|t| t == "done");

        if let (Some(in_idx), Some(do_idx), Some(done_idx)) = (in_pos, do_pos, done_pos) {
            let var_name = &tokens[1];
            let items = &tokens[in_idx + 1..do_idx];
            let inner_commands = &tokens[do_idx + 1..done_idx];

            let mut last_status = true;

            for item in items {
                std::env::set_var(var_name, expand_word(state, item));
                last_status = evaluate_tokens(state, inner_commands);
            }

            return last_status;
        } else {
            eprintln!("rsh: syntax error in for loop");
            return false;
        }
    }

    let logical_groups = parse_logic(state, tokens);
    let mut skip = false;
    let mut last_success = true;

    for group in logical_groups {
        if group.pipeline.is_empty()
            || (group.pipeline.len() == 1 && group.pipeline[0].command.is_empty())
        {
            continue;
        }

        if skip {
            match group.next_op {
                Operator::And => skip = true,
                Operator::Or => skip = false,
                Operator::Async => skip = false,
                Operator::None => {}
            }
            continue;
        }

        let is_background = group.next_op == Operator::Async;

        last_success = if group.pipeline.len() == 1 {
            execute_single(state, &group.pipeline[0], is_background)
        } else {
            execute_pipeline(state, &group.pipeline, is_background)
        };

        match group.next_op {
            Operator::And => skip = !last_success,
            Operator::Or => skip = last_success,
            Operator::Async => skip = false,
            Operator::None => {}
        }
    }

    last_success
}

fn main() {
    let mut state = ShellState::new();
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let history_file = PathBuf::from(&home_dir).join(".rsh_history");
    let mut rl = DefaultEditor::new().expect("Failed to create readline editor");
    let _ = rl.load_history(&history_file);

    let rc_file = PathBuf::from(&home_dir).join(".rshrc");
    if let Ok(contents) = std::fs::read_to_string(&rc_file) {
        for line in contents.lines() {
            let trimmed = line.trim();
            // Ignore empty lines and comments!
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                let tokens = tokenize(trimmed);
                evaluate_tokens(&mut state, &tokens);
            }
        }
    }
    ctrlc::set_handler(move || {
        println!();
    })
    .expect("Error setting Ctrl-C handler");

    // --- STAGE 20: TERMINAL INDEPENDENCE (Inside main!) ---
    unsafe {
        // 1. Ignore background read/write signals
        libc::signal(libc::SIGTTOU, libc::SIG_IGN);
        libc::signal(libc::SIGTTIN, libc::SIG_IGN);

        // 2. IMPORTANT: Tell the OS that Ctrl-Z should NEVER suspend the shell itself!
        libc::signal(libc::SIGTSTP, libc::SIG_IGN);

        // 3. Put the shell into its own isolated process group
        let shell_pgid = libc::getpid();
        if libc::setpgid(shell_pgid, shell_pgid) < 0 {
            eprintln!("Warning: Failed to put shell in its own process group.");
        }

        // 4. Seize absolute control of the terminal keyboard
        libc::tcsetpgrp(libc::STDIN_FILENO, shell_pgid);
    }

    // Handle Subshell / Script execution (-c flag)
    // ...    // Handle Subshell / Script execution (-c flag)
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "-c" {
        let tokens = tokenize(&args[2]);
        evaluate_tokens(&mut state, &tokens);
        return;
    }

    // Determine where to save the history file (~/.rsh_history)
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let history_file = PathBuf::from(&home_dir).join(".rsh_history");

    // Silently load existing history if it exists
    let _ = rl.load_history(&history_file);

    loop {
        // rustyline handles printing the prompt and reading the line
        let readline = rl.readline("$ ");

        match readline {
            Ok(line) => {
                let trimmed_input = line.trim();

                if trimmed_input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(trimmed_input);

                let _ = rl.save_history(&history_file);

                let tokens = tokenize(trimmed_input);
                evaluate_tokens(&mut state, &tokens);
            }
            Err(ReadlineError::Interrupted) => {
                continue;
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

fn execute_single(state: &mut ShellState, expr: &Command, background: bool) -> bool {
    let mut output: Box<dyn Write> = if let Some(file) = &expr.stdout_file {
        if expr.append_stdout {
            Box::new(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file)
                    .unwrap(),
            )
        } else {
            Box::new(File::create(file).unwrap())
        }
    } else {
        Box::new(std::io::stdout())
    };

    let mut err_output: Box<dyn Write> = if let Some(file) = &expr.stderr_file {
        if expr.append_stderr {
            Box::new(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file)
                    .unwrap(),
            )
        } else {
            Box::new(File::create(file).unwrap())
        }
    } else {
        Box::new(std::io::stderr())
    };

    // --- STAGE 17: ALIAS EXPANSION ---
    let mut cmd_name = expr.command.clone();
    let mut cmd_args = expr.args.clone();

    if let Some(expanded) = state.aliases.get(&cmd_name) {
        let mut parts: Vec<String> = expanded.split_whitespace().map(String::from).collect();
        if !parts.is_empty() {
            cmd_name = parts.remove(0);
            parts.extend(cmd_args);
            cmd_args = parts;
        }
    }

    // Pass the expanded cmd_name and cmd_args to Builtin::parse
    if let Some(builtin) = Builtin::parse(&cmd_name, &cmd_args) {
        let builtin_success = match builtin {
            Builtin::Exit(code) => std::process::exit(code),
            Builtin::Echo(args) => {
                writeln!(output, "{}", args.join(" ")).unwrap();
                true
            }
            Builtin::Type(commands) => {
                for cmd in commands {
                    match cmd.as_str() {
                        "echo" | "exit" | "type" | "cd" | "pwd" | "export" | "alias" | "jobs"
                        | "fg" | "bg" => writeln!(output, "{} is a shell builtin", cmd).unwrap(),
                        _ => match find_in_path(&cmd) {
                            Some(full_cmd) => {
                                writeln!(output, "{} is {}", cmd, full_cmd.display()).unwrap()
                            }
                            None => writeln!(output, "{}: not found", cmd).unwrap(),
                        },
                    }
                }
                true
            }
            Builtin::Pwd => {
                writeln!(output, "{}", std::env::current_dir().unwrap().display()).unwrap();
                true
            }
            Builtin::Export(key, value) => {
                std::env::set_var(key, value);
                true
            }
            Builtin::Jobs => {
                state.jobs.retain(|job| {
                    let mut status = 0;
                    let res = unsafe { libc::waitpid(job.pgid, &mut status, libc::WNOHANG) };
                    res == 0
                });

                for job in &state.jobs {
                    let status_str = match job.status {
                        JobStatus::Running => "Running",
                        JobStatus::Stopped => "Stopped",
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
            Builtin::Cd(path) => match std::env::set_current_dir(&path) {
                Ok(_) => true,
                Err(_) => {
                    writeln!(err_output, "cd: {}: No such file or directory", path).unwrap();
                    false
                }
            },
            Builtin::Alias(args) => {
                if args.is_empty() {
                    for (k, v) in &state.aliases {
                        writeln!(output, "alias {}='{}'", k, v).unwrap();
                    }
                } else {
                    for arg in args {
                        if let Some((key, value)) = arg.split_once('=') {
                            let clean_val = value.trim_matches(|c| c == '\'' || c == '"');
                            state.aliases.insert(key.to_string(), clean_val.to_string());
                        }
                    }
                }
                true
            }
        };
        // Save the builtin's exit status!
        state.last_exit_status = if builtin_success { 0 } else { 1 };
        builtin_success
    } else {
        if let Some(full_command) = find_in_path(&cmd_name) {
            let mut child = std::process::Command::new(full_command);
            child.args(&cmd_args);

            // --- STANDARD INPUT REDIRECTION (<) ---
            if let Some(in_file) = &expr.stdin_file {
                if let Ok(file) = std::fs::File::open(in_file) {
                    child.stdin(std::process::Stdio::from(file));
                } else {
                    eprintln!("{}: No such file or directory", in_file);
                    state.last_exit_status = 1;
                    return false;
                }
            }

            // --- STANDARD OUTPUT REDIRECTION (>, >>) ---
            if let Some(out_file) = &expr.stdout_file {
                let file = if expr.append_stdout {
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(out_file)
                        .unwrap()
                } else {
                    File::create(out_file).unwrap()
                };
                child.stdout(std::process::Stdio::from(file));
            }

            // --- STANDARD ERROR REDIRECTION (2>, 2>>) ---
            if let Some(err_file) = &expr.stderr_file {
                let file = if expr.append_stderr {
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(err_file)
                        .unwrap()
                } else {
                    File::create(err_file).unwrap()
                };
                child.stderr(std::process::Stdio::from(file));
            }

            unsafe {
                child.pre_exec(|| {
                    libc::setpgid(0, 0);
                    libc::signal(libc::SIGINT, libc::SIG_DFL);
                    libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                    libc::signal(libc::SIGTSTP, libc::SIG_DFL);
                    libc::signal(libc::SIGTTIN, libc::SIG_DFL);
                    libc::signal(libc::SIGTTOU, libc::SIG_DFL);
                    Ok(())
                });
            }

            match child.spawn() {
                Ok(spawned) => {
                    let pid = spawned.id() as i32;
                    let pgid = pid;

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
                        state.last_exit_status = 0; // Background jobs don't block
                        true
                    } else {
                        unsafe {
                            libc::tcsetpgrp(libc::STDIN_FILENO, pgid);
                        }

                        let mut status: libc::c_int = 0;
                        unsafe {
                            libc::waitpid(pgid, &mut status, libc::WUNTRACED);
                        }
                        unsafe {
                            libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpid());
                        }

                        if libc::WIFSTOPPED(status) {
                            let job_id = state.job_id_counter;
                            state.jobs.push(Job {
                                id: job_id,
                                pgid,
                                command: format!("{} {}", cmd_name, cmd_args.join(" ")),
                                status: JobStatus::Stopped,
                            });
                            println!("\n[{}] + Stopped          {}", job_id, cmd_name);
                            state.job_id_counter += 1;

                            // Typical shell behavior for SIGTSTP
                            state.last_exit_status = 148;
                            true
                        } else {
                            if libc::WIFEXITED(status) {
                                state.last_exit_status = libc::WEXITSTATUS(status);
                            } else {
                                state.last_exit_status = 1;
                            }
                            state.last_exit_status == 0
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: {}", cmd_name, e);
                    state.last_exit_status = 126;
                    false
                }
            }
        } else {
            println!("{}: command not found", cmd_name);
            state.last_exit_status = 127;
            false
        }
    }
}

fn execute_pipeline(state: &mut ShellState, pipeline: &[Command], background: bool) -> bool {
    let mut previous_stdout: Option<std::process::ChildStdout> = None;
    let mut builtin_buffer: Option<Vec<u8>> = None;
    let mut final_success = true;

    for (i, cmd) in pipeline.iter().enumerate() {
        let is_last = i == pipeline.len() - 1;

        let mut cmd_name = cmd.command.clone();
        let mut cmd_args = cmd.args.clone();

        if let Some(expanded) = state.aliases.get(&cmd_name) {
            let mut parts: Vec<String> = expanded.split_whitespace().map(String::from).collect();
            if !parts.is_empty() {
                cmd_name = parts.remove(0);
                parts.extend(cmd_args);
                cmd_args = parts;
            }
        }

        if let Some(builtin) = Builtin::parse(&cmd_name, &cmd_args) {
            let mut output = Vec::new();
            match builtin {
                Builtin::Echo(args) => writeln!(output, "{}", args.join(" ")).unwrap(),
                Builtin::Pwd => {
                    writeln!(output, "{}", std::env::current_dir().unwrap().display()).unwrap()
                }
                Builtin::Type(commands) => {
                    for type_cmd in commands {
                        match type_cmd.as_str() {
                            "echo" | "exit" | "type" | "cd" | "pwd" | "export" | "alias"
                            | "jobs" | "fg" | "bg" => {
                                writeln!(output, "{} is a shell builtin", type_cmd).unwrap()
                            }
                            _ => match find_in_path(&type_cmd) {
                                Some(full_cmd) => {
                                    writeln!(output, "{} is {}", type_cmd, full_cmd.display())
                                        .unwrap()
                                }
                                None => writeln!(output, "{}: not found", type_cmd).unwrap(),
                            },
                        }
                    }
                }
                Builtin::Alias(alias_args) => {
                    if alias_args.is_empty() {
                        for (k, v) in &state.aliases {
                            writeln!(output, "alias {}='{}'", k, v).unwrap();
                        }
                    } else {
                        for arg in alias_args {
                            if let Some((key, value)) = arg.split_once('=') {
                                let clean_val = value.trim_matches(|c| c == '\'' || c == '"');
                                state.aliases.insert(key.to_string(), clean_val.to_string());
                            }
                        }
                    }
                }
                Builtin::Jobs => {
                    state.jobs.retain(|job| {
                        let mut status = 0;
                        let res = unsafe { libc::waitpid(job.pgid, &mut status, libc::WNOHANG) };
                        res == 0
                    });
                    for job in &state.jobs {
                        let status_str = match job.status {
                            JobStatus::Running => "Running",
                            JobStatus::Stopped => "Stopped",
                        };
                        writeln!(output, "[{}]  {}    {}", job.id, status_str, job.command)
                            .unwrap();
                    }
                }
                Builtin::Fg(_) | Builtin::Bg(_) => {
                    writeln!(output, "rsh: fg/bg not supported inside pipelines").unwrap();
                }
                Builtin::Cd(_) | Builtin::Exit(_) | Builtin::Export(_, _) => {}
            }

            if is_last {
                std::io::stdout().write_all(&output).unwrap();
                state.last_exit_status = 0;
                final_success = true;
            } else {
                builtin_buffer = Some(output);
            }
        } else {
            if let Some(full_command) = find_in_path(&cmd_name) {
                let mut child = std::process::Command::new(full_command);
                child.args(&cmd_args);

                if let Some(out) = previous_stdout.take() {
                    child.stdin(std::process::Stdio::from(out));
                } else if let Some(buf) = builtin_buffer.take() {
                    child.stdin(std::process::Stdio::piped());
                    builtin_buffer = Some(buf);
                }

                if !is_last {
                    child.stdout(std::process::Stdio::piped());
                }

                if let Some(err_file) = &cmd.stderr_file {
                    let file = if cmd.append_stderr {
                        OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(err_file)
                            .unwrap()
                    } else {
                        File::create(err_file).unwrap()
                    };
                    child.stderr(std::process::Stdio::from(file));
                }

                let mut spawned = child.spawn().expect("failed to spawn");

                if let Some(buf) = builtin_buffer.take() {
                    if let Some(mut stdin) = spawned.stdin.take() {
                        stdin.write_all(&buf).unwrap();
                    }
                }

                if !is_last {
                    previous_stdout = spawned.stdout.take();
                } else {
                    if background {
                        println!("[1] {}", spawned.id());
                        state.last_exit_status = 0;
                        final_success = true;
                    } else {
                        let status = spawned.wait().map(|s| s.code().unwrap_or(1)).unwrap_or(1);
                        state.last_exit_status = status;
                        final_success = status == 0;
                    }
                }
            } else {
                println!("{}: command not found", cmd_name);
                state.last_exit_status = 127;
                return false;
            }
        }
    }
    final_success
}

fn find_in_path(command: &str) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    for path in std::env::split_paths(&path_var) {
        let full_path = path.join(command);
        if full_path.is_file() {
            return Some(full_path);
        }
    }
    None
}

pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    // NEW: Track subshell depth so we don't split spaces inside $(...)
    let mut subshell_depth = 0;

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if escaped {
            current_token.push(c);
            escaped = false;
            i += 1;
            continue;
        }

        match c {
            '\\' => {
                escaped = true;
                current_token.push(c);
            }
            '\'' if !in_double => {
                in_single = !in_single;
                current_token.push(c);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current_token.push(c);
            }
            '$' if !in_single => {
                current_token.push(c);
                // Check if a subshell is starting
                if i + 1 < chars.len() && chars[i + 1] == '(' {
                    current_token.push('(');
                    subshell_depth += 1;
                    i += 2;
                    continue;
                }
            }
            '(' if subshell_depth > 0 && !in_single && !in_double => {
                subshell_depth += 1; // Handle nested subshells like $(echo $(ls))
                current_token.push(c);
            }
            ')' if subshell_depth > 0 && !in_single && !in_double => {
                subshell_depth -= 1;
                current_token.push(c);
            }
            ' ' | '\t' | '\n' if !in_single && !in_double && subshell_depth == 0 => {
                // Only split on spaces if we are NOT in quotes and NOT in a subshell
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            _ => {
                current_token.push(c);
            }
        }
        i += 1;
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    tokens
}
fn eval_math(expr: &str) -> Result<i64, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.chars().filter(|c| !c.is_whitespace()).collect();
    let mut i = 0;

    // Phase 1: Lexer
    while i < chars.len() {
        match chars[i] {
            '+' | '-' | '*' | '/' | '(' | ')' => {
                tokens.push(chars[i].to_string());
                i += 1;
            }
            '0'..='9' => {
                let mut num = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    num.push(chars[i]);
                    i += 1;
                }
                tokens.push(num);
            }
            _ => return Err(format!("Invalid math char: {}", chars[i])),
        }
    }

    // Phase 2: Shunting-Yard (Infix to Postfix)
    let mut rpn = Vec::new();
    let mut ops: Vec<String> = Vec::new();

    let precedence = |op: &str| -> i32 {
        match op {
            "+" | "-" => 1,
            "*" | "/" => 2,
            _ => 0,
        }
    };

    for token in tokens {
        if token.parse::<i64>().is_ok() {
            rpn.push(token);
        } else if token == "(" {
            ops.push(token);
        } else if token == ")" {
            while let Some(op) = ops.pop() {
                if op == "(" {
                    break;
                }
                rpn.push(op);
            }
        } else {
            while let Some(top_op) = ops.last() {
                if precedence(top_op) >= precedence(&token) {
                    rpn.push(ops.pop().unwrap());
                } else {
                    break;
                }
            }
            ops.push(token);
        }
    }
    while let Some(op) = ops.pop() {
        if op == "(" {
            return Err("Mismatched parentheses".to_string());
        }
        rpn.push(op);
    }

    // Phase 3: Stack Machine Evaluator
    let mut stack: Vec<i64> = Vec::new();
    for token in rpn {
        if let Ok(num) = token.parse::<i64>() {
            stack.push(num);
        } else {
            let b = stack.pop().ok_or("Invalid math expression")?;
            let a = stack.pop().ok_or("Invalid math expression")?;
            let res = match token.as_str() {
                "+" => a + b,
                "-" => a - b,
                "*" => a * b,
                "/" => {
                    if b == 0 {
                        return Err("Division by zero".to_string());
                    }
                    a / b
                }
                _ => return Err("Unknown operator".to_string()),
            };
            stack.push(res);
        }
    }

    stack.pop().ok_or("Empty expression".to_string())
}
