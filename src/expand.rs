use crate::state::ShellState;
use std::path::PathBuf;

// ── Glob / pattern matching ───────────────────────────────────────────────────

pub fn match_pattern(pattern: &str, text: &str) -> bool {
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

pub fn expand_glob(word: &str) -> Vec<String> {
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

// ── Word expansion ($VAR, $(...), $((...)), ${VAR:-default}) ──────────────────

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
                    result.push(c);
                } else {
                    escaped = true;
                }
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }

            '$' if !in_single => {
                // Arithmetic expansion: $((expr))
                if i + 2 < chars.len() && chars[i + 1] == '(' && chars[i + 2] == '(' {
                    i += 3;
                    let mut math_expr = String::new();
                    while i + 1 < chars.len() && !(chars[i] == ')' && chars[i + 1] == ')') {
                        math_expr.push(chars[i]);
                        i += 1;
                    }
                    i += 2;
                    match eval_math(&math_expr) {
                        Ok(num) => result.push_str(&num.to_string()),
                        Err(e) => eprintln!("rsh: math error: {}", e),
                    }
                    continue;
                }

                // Command substitution: $(cmd)
                if i + 1 < chars.len() && chars[i + 1] == '(' {
                    i += 2;
                    let mut sub_cmd = String::new();
                    let mut depth = 1;
                    while i < chars.len() && depth > 0 {
                        if chars[i] == '(' {
                            depth += 1;
                        } else if chars[i] == ')' {
                            depth -= 1;
                        }
                        if depth > 0 {
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
                            let out = String::from_utf8_lossy(&output.stdout);
                            result.push_str(out.trim_end_matches('\n'));
                        }
                    }
                    continue;
                }

                // Exit status: $?
                if i + 1 < chars.len() && chars[i + 1] == '?' {
                    result.push_str(&state.last_exit_status.to_string());
                    i += 2;
                    continue;
                }

                // Brace expansion: ${VAR} or ${VAR:-default}
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    i += 2;
                    let mut inside = String::new();
                    while i < chars.len() && chars[i] != '}' {
                        inside.push(chars[i]);
                        i += 1;
                    }
                    if i < chars.len() {
                        i += 1;
                    } // skip '}'

                    if let Some((var, default)) = inside.split_once(":-") {
                        match std::env::var(var) {
                            Ok(val) if !val.is_empty() => result.push_str(&val),
                            _ => result.push_str(default),
                        }
                    } else {
                        if let Ok(val) = std::env::var(&inside) {
                            result.push_str(&val);
                        }
                    }
                    continue;
                }

                // Plain variable: $VAR
                i += 1;
                let mut var_name = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    var_name.push(chars[i]);
                    i += 1;
                }
                if var_name.is_empty() {
                    result.push('$');
                } else if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                }
                continue;
            }

            _ => result.push(c),
        }
        i += 1;
    }

    result
}

// ── PATH lookup ───────────────────────────────────────────────────────────────

pub fn find_in_path(command: &str) -> Option<PathBuf> {
    if command.starts_with("./") || command.starts_with("../") || command.starts_with('/') {
        let path = PathBuf::from(command);
        return if path.is_file() { Some(path) } else { None };
    }
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path_var) {
        let full = dir.join(command);
        if full.is_file() {
            return Some(full);
        }
    }
    None
}

// ── Arithmetic evaluator (shunting-yard → RPN) ────────────────────────────────

pub fn eval_math(expr: &str) -> Result<i64, String> {
    let chars: Vec<char> = expr.chars().filter(|c| !c.is_whitespace()).collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    // Lexer
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
            c => return Err(format!("Invalid math char: {}", c)),
        }
    }

    // Shunting-yard (infix → RPN)
    let precedence = |op: &str| -> i32 {
        match op {
            "+" | "-" => 1,
            "*" | "/" => 2,
            _ => 0,
        }
    };

    let mut rpn: Vec<String> = Vec::new();
    let mut ops: Vec<String> = Vec::new();

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
            while ops
                .last()
                .map(|o: &String| precedence(o) >= precedence(&token))
                .unwrap_or(false)
            {
                rpn.push(ops.pop().unwrap());
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

    // Stack-machine evaluator
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
pub fn is_tty(fd: libc::c_int) -> bool {
    unsafe { libc::isatty(fd) == 1 }
}
