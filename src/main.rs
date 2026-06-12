use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

struct ShellState {
    aliases: HashMap<String, String>,
}

impl ShellState {
    fn new() -> Self {
        ShellState {
            aliases: HashMap::new(),
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
    fn from_tokens(tokens: Vec<String>) -> Command {
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
                        stdin_file = Some(expand_word(&tokens[i + 1]));
                        i += 1;
                    }
                }
                ">" | "1>" => {
                    if i + 1 < tokens.len() {
                        stdout_file = Some(expand_word(&tokens[i + 1]));
                        append_stdout = false;
                        i += 1;
                    }
                }
                ">>" | "1>>" => {
                    if i + 1 < tokens.len() {
                        stdout_file = Some(expand_word(&tokens[i + 1]));
                        append_stdout = true;
                        i += 1;
                    }
                }

                "2>" => {
                    if i + 1 < tokens.len() {
                        stderr_file = Some(expand_word(&tokens[i + 1]));
                        append_stderr = false;
                        i += 1;
                    }
                }
                "2>>" => {
                    if i + 1 < tokens.len() {
                        stderr_file = Some(expand_word(&tokens[i + 1]));
                        append_stderr = true;
                        i += 1;
                    }
                }
                _ => {
                    let expanded = expand_word(&tokens[i]);
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

fn expand_word(word: &str) -> String {
    let mut result = String::new();
    let mut chars = word.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '$' if !in_single => {
                if let Some(&'(') = chars.peek() {
                    chars.next();
                    let mut inner_cmd = String::new();
                    let mut paren_count = 1;

                    while let Some(inner_c) = chars.next() {
                        if inner_c == '(' {
                            paren_count += 1;
                        } else if inner_c == ')' {
                            paren_count -= 1;
                            if paren_count == 0 {
                                break;
                            }
                        }
                        inner_cmd.push(inner_c);
                    }

                    if let Ok(exe) = std::env::current_exe() {
                        if let Ok(output) = std::process::Command::new(exe)
                            .arg("-c")
                            .arg(&inner_cmd)
                            .output()
                        {
                            let out_str = String::from_utf8_lossy(&output.stdout);
                            result.push_str(out_str.trim_end_matches(|c| c == '\n' || c == '\r'));
                        }
                    }
                } else {
                    let mut var_name = String::new();
                    while let Some(&next_c) = chars.peek() {
                        if next_c.is_alphanumeric() || next_c == '_' {
                            var_name.push(next_c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if !var_name.is_empty() {
                        if let Ok(val) = std::env::var(&var_name) {
                            result.push_str(&val);
                        }
                    } else {
                        result.push('$');
                    }
                }
            }
            _ => result.push(c),
        }
    }
    result
}

fn parse_logic(tokens: &[String]) -> Vec<LogicalGroup> {
    let mut groups = Vec::new();
    let mut current_tokens = Vec::new();

    for token in tokens {
        match token.as_str() {
            "&&" => {
                let pipeline = parse_pipeline_from_tokens(&current_tokens);
                groups.push(LogicalGroup {
                    pipeline,
                    next_op: Operator::And,
                });
                current_tokens.clear();
            }
            "||" => {
                let pipeline = parse_pipeline_from_tokens(&current_tokens);
                groups.push(LogicalGroup {
                    pipeline,
                    next_op: Operator::Or,
                });
                current_tokens.clear();
            }
            "&" => {
                let pipeline = parse_pipeline_from_tokens(&current_tokens);
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
        let pipeline = parse_pipeline_from_tokens(&current_tokens);
        groups.push(LogicalGroup {
            pipeline,
            next_op: Operator::None,
        });
    }

    groups
}

fn parse_pipeline_from_tokens(tokens: &[String]) -> Vec<Command> {
    let mut commands = Vec::new();
    let mut current_tokens = Vec::new();

    for token in tokens {
        if token == "|" {
            commands.push(Command::from_tokens(current_tokens.clone()));
            current_tokens.clear();
        } else {
            current_tokens.push(token.clone());
        }
    }
    commands.push(Command::from_tokens(current_tokens));
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
                std::env::set_var(var_name, expand_word(item));
                last_status = evaluate_tokens(state, inner_commands);
            }

            return last_status;
        } else {
            eprintln!("rsh: syntax error in for loop");
            return false;
        }
    }

    let logical_groups = parse_logic(tokens);
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

    // Handle Subshell / Script execution (-c flag)
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
        match builtin {
            Builtin::Exit(code) => std::process::exit(code),
            Builtin::Echo(args) => {
                writeln!(output, "{}", args.join(" ")).unwrap();
                true
            }
            Builtin::Type(commands) => {
                for cmd in commands {
                    match cmd.as_str() {
                        "echo" | "exit" | "type" | "cd" | "pwd" | "export" | "alias" => {
                            writeln!(output, "{} is a shell builtin", cmd).unwrap()
                        }
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
            Builtin::Cd(path) => match std::env::set_current_dir(&path) {
                Ok(_) => true,
                Err(_) => {
                    writeln!(err_output, "cd: {}: No such file or directory", path).unwrap();
                    false
                }
            },
            // NEW: The Alias Builtin Logic
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
        }
    } else {
        // Use cmd_name and cmd_args for external binaries!
        if let Some(full_command) = find_in_path(&cmd_name) {
            let mut child = std::process::Command::new(full_command);
            child.args(&cmd_args);

            if let Some(in_file) = &expr.stdin_file {
                if let Ok(file) = File::open(in_file) {
                    child.stdin(std::process::Stdio::from(file));
                } else {
                    eprintln!("{}: No such file or directory", in_file);
                    return false;
                }
            }
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

            match child.spawn() {
                Ok(mut spawned) => {
                    if background {
                        println!("[1] {}", spawned.id());
                        true
                    } else {
                        spawned.wait().map(|s| s.success()).unwrap_or(false)
                    }
                }
                Err(e) => {
                    eprintln!("{}: {}", cmd_name, e);
                    false
                }
            }
        } else {
            println!("{}: command not found", cmd_name);
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

        // --- STAGE 17: ALIAS EXPANSION ---
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
                            "echo" | "exit" | "type" | "cd" | "pwd" | "export" | "alias" => {
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
                Builtin::Cd(_) | Builtin::Exit(_) | Builtin::Export(_, _) => {}
            }

            if is_last {
                std::io::stdout().write_all(&output).unwrap();
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
                        final_success = true;
                    } else {
                        final_success = spawned.wait().map(|s| s.success()).unwrap_or(false);
                    }
                }
            } else {
                println!("{}: command not found", cmd_name);
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

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut subshell_depth = 0;

    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double_quote && subshell_depth == 0 => {
                in_single_quote = !in_single_quote;
                current_token.push(c);
            }
            '"' if !in_single_quote && subshell_depth == 0 => {
                in_double_quote = !in_double_quote;
                current_token.push(c);
            }
            '$' if !in_single_quote => {
                current_token.push(c);
                if let Some(&'(') = chars.peek() {
                    chars.next();
                    current_token.push('(');
                    subshell_depth += 1;
                }
            }
            '(' if subshell_depth > 0 => {
                current_token.push(c);
                subshell_depth += 1;
            }
            ')' if subshell_depth > 0 => {
                current_token.push(c);
                subshell_depth -= 1;
            }
            ' ' if !in_single_quote && !in_double_quote && subshell_depth == 0 => {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            '|' if !in_single_quote && !in_double_quote && subshell_depth == 0 => {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                if let Some(&'|') = chars.peek() {
                    chars.next();
                    tokens.push("||".to_string());
                } else {
                    tokens.push("|".to_string());
                }
            }
            '&' if !in_single_quote && !in_double_quote && subshell_depth == 0 => {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                if let Some(&'&') = chars.peek() {
                    chars.next();
                    tokens.push("&&".to_string());
                } else {
                    tokens.push("&".to_string());
                }
            }
            _ => current_token.push(c),
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token);
    }
    tokens
}
