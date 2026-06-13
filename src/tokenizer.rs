pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut cmd_subst_depth = 0;
    let mut in_comment = false; // NEW: Tracks comment state

    for c in input.chars() {
        // 1. If we are in a comment, ignore everything until the newline
        if in_comment {
            if c == '\n' {
                in_comment = false;
                // A newline after a comment still acts as a statement separator!
                if cmd_subst_depth == 0 {
                    if let Some(last) = tokens.last() {
                        if last != ";" && last != "&&" && last != "||" && last != "|" {
                            tokens.push(";".to_string());
                        }
                    }
                }
            }
            continue;
        }

        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }

        if c == '\\' && !in_single {
            current.push(c);
            escaped = true;
            continue;
        }

        if c == '\'' && !in_double {
            in_single = !in_single;
            current.push(c);
            continue;
        }

        if c == '"' && !in_single {
            in_double = !in_double;
            current.push(c);
            continue;
        }

        if !in_single && !in_double {
            if c == '(' && current.ends_with('$') {
                cmd_subst_depth += 1;
            } else if c == '(' && cmd_subst_depth > 0 {
                cmd_subst_depth += 1;
            } else if c == ')' && cmd_subst_depth > 0 {
                cmd_subst_depth -= 1;
            }

            // Enter comment state
            if c == '#' && current.is_empty() {
                in_comment = true;
                continue;
            }

            // NEW: Translate Newlines into Semicolons
            if c == '\n' && cmd_subst_depth == 0 {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                // Prevent double semicolons or putting a semicolon after a pipe
                if let Some(last) = tokens.last() {
                    if last != ";" && last != "&&" && last != "||" && last != "|" {
                        tokens.push(";".to_string());
                    }
                }
                continue;
            }
            if c == ';' && cmd_subst_depth == 0 {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                // Only push a semicolon if the last token isn't already one
                if tokens.last().map(|s| s.as_str()) != Some(";") {
                    tokens.push(";".to_string());
                }
                continue;
            }
            // Normal Whitespace
            if c.is_whitespace() && cmd_subst_depth == 0 {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                continue;
            }
        }

        current.push(c);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

pub fn is_incomplete(input: &str, tokens: &[String]) -> bool {
    // 1. Check for trailing pipes or logical operators
    if let Some(last) = tokens.last() {
        let s = last.as_str();
        if s == "|" || s == "&&" || s == "||" {
            return true;
        }
    }

    // 2. Check for unclosed quotes or command substitutions
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut cmd_subst_depth = 0;
    let mut last_char = '\0';

    for c in input.chars() {
        if escaped {
            escaped = false;
            last_char = c;
            continue;
        }
        if c == '\\' && !in_single {
            escaped = true;
            last_char = c;
            continue;
        }
        if c == '\'' && !in_double {
            in_single = !in_single;
            last_char = c;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
            last_char = c;
            continue;
        }

        if !in_single && !in_double {
            if c == '(' && last_char == '$' {
                cmd_subst_depth += 1;
            } else if c == '(' && cmd_subst_depth > 0 {
                cmd_subst_depth += 1;
            } else if c == ')' && cmd_subst_depth > 0 {
                cmd_subst_depth -= 1;
            }
        }
        last_char = c;
    }

    // 3. NEW: Check for unclosed AST control flow blocks!
    let mut block_depth = 0;
    for t in tokens {
        match t.as_str() {
            "if" | "for" | "while" | "{" | "(" => block_depth += 1,
            "fi" | "done" | "}" | ")" => block_depth -= 1,
            _ => {}
        }
    }

    // If ANY of these are true, the user needs to keep typing
    in_single || in_double || escaped || cmd_subst_depth > 0 || block_depth > 0
}
