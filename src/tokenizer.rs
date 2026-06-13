/// Turn a raw input string into a list of shell tokens.
/// Handles single/double quoting, backslash escapes, and subshell depth
/// so spaces inside `$(...)` are not treated as delimiters.
pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut subshell_depth: usize = 0;

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if escaped {
            current.push(c);
            escaped = false;
            i += 1;
            continue;
        }

        match c {
            '\\' => {
                escaped = true;
                current.push(c);
            }
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(c);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(c);
            }
            '$' if !in_single => {
                current.push(c);
                if i + 1 < chars.len() && chars[i + 1] == '(' {
                    current.push('(');
                    subshell_depth += 1;
                    i += 2;
                    continue;
                }
            }
            '(' if subshell_depth > 0 && !in_single => {
                subshell_depth += 1;
                current.push(c);
            }
            ')' if subshell_depth > 0 && !in_single => {
                subshell_depth -= 1;
                current.push(c);
            }
            // Token delimiters (only at top level, outside quotes)
            ' ' | '\t' | '\n' | ';' if !in_single && !in_double && subshell_depth == 0 => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                // Newlines and semicolons become explicit statement separators
                if c == '\n' || c == ';' {
                    if tokens.last().map(|s: &String| s.as_str()) != Some(";") {
                        tokens.push(";".to_string());
                    }
                }
            }
            _ => current.push(c),
        }
        i += 1;
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Returns `true` when the user's input is structurally incomplete (open
/// keyword, unclosed quote, trailing backslash, etc.) so the REPL can
/// prompt for a continuation line instead of executing prematurely.
pub fn is_incomplete(input: &str, tokens: &[String]) -> bool {
    // 1. Unmatched block keywords
    let mut depth: i32 = 0;
    for t in tokens {
        match t.as_str() {
            "if" | "for" | "while" | "{" => depth += 1,
            "fi" | "done" | "}" => depth -= 1,
            _ => {}
        }
    }
    if depth > 0 {
        return true;
    }

    // 2. Unclosed quotes / subshells / trailing backslash
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut paren_depth: i32 = 0;

    for (i, c) in input.chars().enumerate() {
        let _ = i; // suppress unused warning
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '$' => {} // handled by the next char check below (we rely on tokenizer for this)
            '(' if paren_depth > 0 && !in_single => paren_depth += 1,
            ')' if paren_depth > 0 && !in_single => paren_depth -= 1,
            _ => {}
        }
    }

    in_single || in_double || paren_depth > 0 || escaped
}
