use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Clone)]
enum Operator {
    And,
    Or,
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
            };
        }

        let mut args = Vec::new();
        let mut stdin_file = None;
        let mut stdout_file = None;
        let mut append_stdout = false;

        let mut i = 0;
        while i < tokens.len() {
            match tokens[i].as_str() {
                "<" => {
                    if i + 1 < tokens.len() {
                        // STAGE 11: Expand the word when reading it!
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

        Command {
            command,
            args,
            stdin_file,
            stdout_file,
            append_stdout,
        }
    }
}

fn expand_word(word: &str) -> String {
    let mut result = String::new();
    let mut chars = word.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => in_single = !in_single, // Strip the single quote
            '"' if !in_single => in_double = !in_double,  // Strip the double quote
            '$' if !in_single => {
                if let Some(&'(') = chars.peek() {
                    chars.next(); // Consume the '('
                    let mut inner_cmd = String::new();
                    let mut paren_count = 1;

                    // Read until we find the matching closing ')'
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

fn evaluate_tokens(tokens: &[String]) -> bool {
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
                // Expand the item BEFORE assigning it to the variable!
                std::env::set_var(var_name, expand_word(item));
                last_status = evaluate_tokens(inner_commands);
            }

            return last_status;
        } else {
            eprintln!(
                "rsh: syntax error in for loop (expected: for VAR in ITEMS do COMMANDS done)"
            );
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
                Operator::None => {}
            }
            continue;
        }

        last_success = if group.pipeline.len() == 1 {
            execute_single(&group.pipeline[0])
        } else {
            execute_pipeline(&group.pipeline)
        };

        match group.next_op {
            Operator::And => skip = !last_success,
            Operator::Or => skip = last_success,
            Operator::None => {}
        }
    }

    last_success
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "-c" {
        // If run with `-c "command"`, just execute that command and exit
        let tokens = tokenize(&args[2]);
        evaluate_tokens(&tokens);
        return;
    }

    loop {
        print!("$ ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let trimmed_input = input.trim();

        if trimmed_input.is_empty() {
            continue;
        }

        let tokens = tokenize(trimmed_input);
        evaluate_tokens(&tokens);
    }
}

fn execute_single(expr: &Command) -> bool {
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

    if let Some(builtin) = Builtin::parse(&expr.command, &expr.args) {
        match builtin {
            Builtin::Exit(code) => std::process::exit(code),
            Builtin::Echo(args) => {
                writeln!(output, "{}", args.join(" ")).unwrap();
                true
            }
            Builtin::Type(commands) => {
                for cmd in commands {
                    match cmd.as_str() {
                        "echo" | "exit" | "type" | "cd" | "pwd" | "export" => {
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
                    println!("cd: {}: No such file or directory", path);
                    false
                }
            },
        }
    } else {
        if let Some(full_command) = find_in_path(&expr.command) {
            let mut child = std::process::Command::new(full_command);
            child.args(&expr.args);

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

            match child.status() {
                Ok(status) => status.success(),
                Err(e) => {
                    eprintln!("{}: {}", expr.command, e);
                    false
                }
            }
        } else {
            println!("{}: command not found", expr.command);
            false
        }
    }
}

fn execute_pipeline(pipeline: &[Command]) -> bool {
    let mut previous_stdout: Option<std::process::ChildStdout> = None;
    let mut builtin_buffer: Option<Vec<u8>> = None;
    let mut final_success = true;

    for (i, cmd) in pipeline.iter().enumerate() {
        let is_last = i == pipeline.len() - 1;

        if let Some(builtin) = Builtin::parse(&cmd.command, &cmd.args) {
            let mut output = Vec::new();
            match builtin {
                Builtin::Echo(args) => writeln!(output, "{}", args.join(" ")).unwrap(),
                Builtin::Pwd => {
                    writeln!(output, "{}", std::env::current_dir().unwrap().display()).unwrap()
                }
                Builtin::Type(commands) => {
                    for cmd in commands {
                        match cmd.as_str() {
                            "echo" | "exit" | "type" | "cd" | "pwd" | "export" => {
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
            if let Some(full_command) = find_in_path(&cmd.command) {
                let mut child = std::process::Command::new(full_command);
                child.args(&cmd.args);

                if let Some(out) = previous_stdout.take() {
                    child.stdin(std::process::Stdio::from(out));
                } else if let Some(buf) = builtin_buffer.take() {
                    child.stdin(std::process::Stdio::piped());
                    builtin_buffer = Some(buf);
                }

                if !is_last {
                    child.stdout(std::process::Stdio::piped());
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
                    final_success = spawned.wait().map(|s| s.success()).unwrap_or(false);
                }
            } else {
                println!("{}: command not found", cmd.command);
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
                    subshell_depth += 1; // Enter subshell parsing
                }
            }
            '(' if subshell_depth > 0 => {
                current_token.push(c);
                subshell_depth += 1; // Handle nested parenthesis like $((1+1))
            }
            ')' if subshell_depth > 0 => {
                current_token.push(c);
                subshell_depth -= 1; // Exit subshell parsing
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

fn match_pattern(pattern: &str, text: &str) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }

    if pattern.starts_with('*') {
        // '*' can match zero characters (skip the '*')
        // OR it can match one character (skip the text char and keep trying the '*')
        match_pattern(&pattern[1..], text)
            || (!text.is_empty() && match_pattern(pattern, &text[1..]))
    } else {
        // Literal character match
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
                // By default, wildcards do not match hidden files (starting with '.') unless explicitly requested
                if name.starts_with('.') && !word.starts_with('.') {
                    continue;
                }

                if match_pattern(word, &name) {
                    matches.push(name);
                }
            }
        }
    }

    // POSIX shell rules: If a glob doesn't match anything, it stays as the literal string (e.g., "*.xyz")
    if matches.is_empty() {
        vec![word.to_string()]
    } else {
        matches.sort();
        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_single_quotes() {
        assert_eq!(
            tokenize("echo 'hello        world'"),
            vec!["echo", "'hello        world'"]
        );
    }

    #[test]
    fn test_var_expansion() {
        std::env::set_var("TEST_VAR", "success");
        assert_eq!(expand_word("$TEST_VAR"), "success");
        assert_eq!(expand_word("\"result: $TEST_VAR\""), "result: success");
        assert_eq!(expand_word("'result: $TEST_VAR'"), "result: $TEST_VAR");
    }

    #[test]
    fn test_tokenize_logical_operators() {
        assert_eq!(
            tokenize("echo first && echo second"),
            vec!["echo", "first", "&&", "echo", "second"]
        );
        assert_eq!(
            tokenize("ls || echo failed"),
            vec!["ls", "||", "echo", "failed"]
        );
    }
}
