use crate::expand::{expand_glob, expand_word};
use crate::state::ShellState;

// ── Builtin commands ──────────────────────────────────────────────────────────

pub enum Builtin {
    Exit(i32),
    Echo(Vec<String>),
    Type(Vec<String>),
    Pwd,
    Cd(Option<String>),
    Alias(Vec<String>),
    Jobs,
    Fg(Option<usize>),
    Bg(Option<usize>),
    RegexMatch(String, String),
    Source(String),
    ReadJson(String),
    Test(Vec<String>),
    Export(Vec<String>),
    Unset(Vec<String>),
    Hash(Vec<String>),
}

impl Builtin {
    pub fn parse(command: &str, args: &[String]) -> Option<Builtin> {
        match command {
            "exit" => {
                let code = args
                    .first()
                    .and_then(|c| c.parse::<i32>().ok())
                    .unwrap_or(0);
                Some(Builtin::Exit(code))
            }
            "echo" => Some(Builtin::Echo(args.to_vec())),
            "type" => Some(Builtin::Type(args.to_vec())),
            "cd" => Some(Builtin::Cd(args.get(0).cloned())),
            "pwd" => Some(Builtin::Pwd),
            "export" => Some(Builtin::Export(args.to_vec())),
            "unset" => Some(Builtin::Unset(args.to_vec())),
            "hash" => Some(Builtin::Hash(args.to_vec())),
            "[[" => {
                // Expected: [[ text =~ pattern ]]
                if args.len() >= 4
                    && args[1] == "=~"
                    && args.last().map(|s| s.as_str()) == Some("]]")
                {
                    Some(Builtin::RegexMatch(args[0].clone(), args[2].clone()))
                } else {
                    None
                }
            }
            "alias" => Some(Builtin::Alias(args.to_vec())),
            "jobs" => Some(Builtin::Jobs),
            "fg" => Some(Builtin::Fg(args.first().and_then(|s| s.parse().ok()))),
            "bg" => Some(Builtin::Bg(args.first().and_then(|s| s.parse().ok()))),
            "source" | "." => Some(Builtin::Source(args.first().cloned().unwrap_or_default())),
            "readjson" => Some(Builtin::ReadJson(args.first().cloned().unwrap_or_default())),
            "test" | "[" => {
                let mut test_args = args.to_vec();
                if command == "[" && test_args.last().map(|s| s.as_str()) == Some("]") {
                    test_args.pop();
                }
                Some(Builtin::Test(test_args))
            }
            _ => None,
        }
    }
}

// ── Command (a single process with its redirections) ──────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub struct Command {
    pub command: String,
    pub args: Vec<String>,
    pub stdin_file: Option<String>,
    pub stdout_file: Option<String>,
    pub append_stdout: bool,
    pub stderr_file: Option<String>,
    pub append_stderr: bool,
    pub merge_stderr: bool,
    pub heredoc_content: Option<String>,
}

impl Command {
    pub fn from_tokens(state: &ShellState, tokens: Vec<String>) -> Command {
        if tokens.is_empty() {
            return Command::empty();
        }

        let mut args = Vec::new();
        let mut stdin_file = None;
        let mut stdout_file = None;
        let mut append_stdout = false;
        let mut stderr_file = None;
        let mut append_stderr = false;
        let mut merge_stderr = false;
        let mut heredoc_content = None;

        let mut i = 0;
        while i < tokens.len() {
            match tokens[i].as_str() {
                "<" if i + 1 < tokens.len() => {
                    stdin_file = Some(expand_word(state, &tokens[i + 1]));
                    i += 1;
                }
                "<<" if i + 1 < tokens.len() => {
                    // We assume the REPL packed the entire multi-line string into this one token!
                    heredoc_content = Some(tokens[i + 1].clone());
                    i += 1;
                }
                ">" | "1>" if i + 1 < tokens.len() => {
                    stdout_file = Some(expand_word(state, &tokens[i + 1]));
                    append_stdout = false;
                    i += 1;
                }
                ">>" | "1>>" if i + 1 < tokens.len() => {
                    stdout_file = Some(expand_word(state, &tokens[i + 1]));
                    append_stdout = true;
                    i += 1;
                }
                "2>" if i + 1 < tokens.len() => {
                    stderr_file = Some(expand_word(state, &tokens[i + 1]));
                    append_stderr = false;
                    i += 1;
                }
                "2>>" if i + 1 < tokens.len() => {
                    stderr_file = Some(expand_word(state, &tokens[i + 1]));
                    append_stderr = true;
                    i += 1;
                }
                "2>&1" => {
                    merge_stderr = true;
                }
                _ => {
                    let expanded = expand_word(state, &tokens[i]);
                    args.extend(expand_glob(&expanded));
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

        // Tilde expansion on args and redirect paths
        let tilde_expand = |s: &mut String| {
            if s == "~" {
                *s = home_dir.clone();
            } else if s.starts_with("~/") {
                *s = s.replacen('~', &home_dir, 1);
            }
        };

        for arg in args.iter_mut() {
            tilde_expand(arg);
        }
        if let Some(f) = stdin_file.as_mut() {
            tilde_expand(f);
        }
        if let Some(f) = stdout_file.as_mut() {
            tilde_expand(f);
        }
        if let Some(f) = stderr_file.as_mut() {
            tilde_expand(f);
        }

        Command {
            command,
            args,
            stdin_file,
            stdout_file,
            append_stdout,
            stderr_file,
            append_stderr,
            merge_stderr,
            heredoc_content,
        }
    }

    fn empty() -> Command {
        Command {
            command: String::new(),
            args: Vec::new(),
            stdin_file: None,
            stdout_file: None,
            append_stdout: false,
            stderr_file: None,
            append_stderr: false,
            merge_stderr: false,
            heredoc_content: None,
        }
    }
}

// ── AST ───────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Clone)]
pub enum ASTNode {
    /// Raw token list — Command parsing is deferred until execution.
    Pipeline(Vec<String>, bool),
    LogicalAnd(Box<ASTNode>, Box<ASTNode>),
    LogicalOr(Box<ASTNode>, Box<ASTNode>),
    FunctionDef {
        name: String,
        body: Box<ASTNode>,
    },
    If {
        condition: Box<ASTNode>,
        then_branch: Box<ASTNode>,
        else_branch: Option<Box<ASTNode>>,
    },
    While {
        condition: Box<ASTNode>,
        body: Box<ASTNode>,
    },
    For {
        var_name: String,
        items: Vec<String>,
        body: Box<ASTNode>,
    },
    Block(Vec<ASTNode>),
}
