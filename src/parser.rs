use crate::state::ShellState;
use crate::types::{ASTNode, Command};

// ── Statement splitting (semicolons, respecting nesting) ──────────────────────

pub fn split_statements(tokens: &[String]) -> Vec<Vec<String>> {
    let mut statements = Vec::new();
    let mut current = Vec::new();
    let mut depth: i32 = 0;

    for t in tokens {
        match t.as_str() {
            "if" | "for" | "while" | "{" | "(" => depth += 1,
            "fi" | "done" | "}" | ")" => depth -= 1,
            _ => {}
        }

        if t == ";" && depth == 0 {
            if !current.is_empty() {
                statements.push(current.clone());
                current.clear();
            }
        } else {
            current.push(t.clone());
        }
    }
    if !current.is_empty() {
        statements.push(current);
    }
    statements
}

// ── Pipeline splitting ────────────────────────────────────────────────────────

pub fn parse_pipeline_from_tokens(state: &ShellState, tokens: &[String]) -> Vec<Command> {
    let mut commands = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if token == "|" {
            commands.push(Command::from_tokens(state, current.clone()));
            current.clear();
        } else {
            current.push(token.clone());
        }
    }
    commands.push(Command::from_tokens(state, current));
    commands
}

// ── Full AST parser ───────────────────────────────────────────────────────────

pub fn parse_ast(state: &ShellState, tokens: &[String]) -> Option<ASTNode> {
    if tokens.is_empty() {
        return None;
    }

    // 1. Multiple statements separated by semicolons
    let statements = split_statements(tokens);
    if statements.len() > 1 {
        let nodes: Vec<ASTNode> = statements
            .into_iter()
            .filter_map(|chunk| parse_ast(state, &chunk))
            .collect();
        return Some(ASTNode::Block(nodes));
    } else if statements.len() == 1 && statements[0].len() < tokens.len() {
        // Strip leading/trailing semicolons
        return parse_ast(state, &statements[0]);
    }

    // 2. Function definitions
    //    Style A: `name() { ... }`
    //    Style B: `name () { ... }`
    let (is_func, func_name, body_start) = detect_function(tokens);
    if is_func {
        let body_tokens = &tokens[body_start..tokens.len() - 1];
        if let Some(body) = parse_ast(state, body_tokens) {
            return Some(ASTNode::FunctionDef {
                name: func_name,
                body: Box::new(body),
            });
        }
    }

    // 3. for … in … do … done
    if tokens[0] == "for" {
        return parse_for(state, tokens);
    }

    // 4. while … do … done
    if tokens[0] == "while" {
        return parse_while(state, tokens);
    }

    // 5. if … then … [else …] fi
    if tokens[0] == "if" {
        return parse_if(state, tokens);
    }

    // 6. Remaining semicolons (shouldn't normally reach here, but kept as safety)
    if tokens.contains(&";".to_string()) {
        let nodes: Vec<ASTNode> = tokens
            .split(|t| t == ";")
            .filter(|chunk| !chunk.is_empty())
            .filter_map(|chunk| parse_ast(state, chunk))
            .collect();
        return Some(ASTNode::Block(nodes));
    }

    // 7. Logical && and ||
    if let Some(pos) = tokens.iter().position(|t| t == "&&") {
        let left = parse_ast(state, &tokens[..pos])?;
        let right = parse_ast(state, &tokens[pos + 1..])?;
        return Some(ASTNode::LogicalAnd(Box::new(left), Box::new(right)));
    }
    if let Some(pos) = tokens.iter().position(|t| t == "||") {
        let left = parse_ast(state, &tokens[..pos])?;
        let right = parse_ast(state, &tokens[pos + 1..])?;
        return Some(ASTNode::LogicalOr(Box::new(left), Box::new(right)));
    }

    // 8. Pipeline / single command
    let is_background = tokens.last().map(|s| s.as_str()) == Some("&");
    let cmd_tokens = if is_background {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };

    if cmd_tokens.is_empty() {
        None
    } else {
        Some(ASTNode::Pipeline(cmd_tokens.to_vec(), is_background))
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn detect_function(tokens: &[String]) -> (bool, String, usize) {
    let last_is_brace = tokens.last().map(|s| s.as_str()) == Some("}");

    // Style A: `name() { … }`
    if tokens.len() >= 3 && tokens[0].ends_with("()") && tokens[1] == "{" && last_is_brace {
        let name = tokens[0].trim_end_matches("()").to_string();
        return (true, name, 2);
    }
    // Style B: `name () { … }`
    if tokens.len() >= 4 && tokens[1] == "()" && tokens[2] == "{" && last_is_brace {
        return (true, tokens[0].clone(), 3);
    }

    (false, String::new(), 0)
}

fn parse_for(state: &ShellState, tokens: &[String]) -> Option<ASTNode> {
    let in_pos = tokens.iter().position(|t| t == "in")?;
    let do_pos = tokens.iter().position(|t| t == "do")?;
    let done_pos = tokens.iter().rposition(|t| t == "done")?;

    if !(in_pos < do_pos && do_pos < done_pos) {
        return None;
    }

    let var_name = tokens[1].clone();
    let items: Vec<String> = tokens[in_pos + 1..do_pos]
        .iter()
        .filter(|t| t.as_str() != ";")
        .cloned()
        .collect();
    let body = Box::new(parse_ast(state, &tokens[do_pos + 1..done_pos])?);

    Some(ASTNode::For {
        var_name,
        items,
        body,
    })
}

fn parse_while(state: &ShellState, tokens: &[String]) -> Option<ASTNode> {
    let do_pos = tokens.iter().position(|t| t == "do")?;
    let done_pos = tokens.iter().rposition(|t| t == "done")?;

    let condition = Box::new(parse_ast(state, &tokens[1..do_pos])?);
    let body = Box::new(parse_ast(state, &tokens[do_pos + 1..done_pos])?);

    Some(ASTNode::While { condition, body })
}

fn parse_if(state: &ShellState, tokens: &[String]) -> Option<ASTNode> {
    let then_pos = tokens.iter().position(|t| t == "then")?;
    let fi_pos = tokens.iter().rposition(|t| t == "fi")?;
    let else_pos = tokens.iter().position(|t| t == "else");

    let condition = Box::new(parse_ast(state, &tokens[1..then_pos])?);

    let (then_branch, else_branch) = if let Some(ep) = else_pos {
        if ep > then_pos && ep < fi_pos {
            let then_b = Box::new(parse_ast(state, &tokens[then_pos + 1..ep])?);
            let else_b = Some(Box::new(parse_ast(state, &tokens[ep + 1..fi_pos])?));
            (then_b, else_b)
        } else {
            return None; // else out of bounds — syntax error
        }
    } else {
        (
            Box::new(parse_ast(state, &tokens[then_pos + 1..fi_pos])?),
            None,
        )
    };

    Some(ASTNode::If {
        condition,
        then_branch,
        else_branch,
    })
}
