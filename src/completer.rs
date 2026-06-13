use rustyline::completion::{Completer, Pair};
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::collections::BTreeSet;

pub struct ShellCompleter {
    pub hinter: HistoryHinter,
    pub highlighter: MatchingBracketHighlighter,
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Only look at what's been typed up to the cursor
        let line = &line[..pos];

        // Find the start of the current word
        let word_start = line
            .rfind(|c: char| c == ' ' || c == '\t')
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &line[word_start..];

        // Are we completing the first token (command position)?
        let is_command = line[..word_start].trim().is_empty();

        let candidates = if is_command {
            complete_command(word)
        } else {
            complete_path(word)
        };

        Ok((word_start, candidates))
    }
}

// ── Command completion (first word) ──────────────────────────────────────────

fn complete_command(prefix: &str) -> Vec<Pair> {
    let mut names: BTreeSet<String> = BTreeSet::new();

    // Built-in commands
    for builtin in &[
        "echo", "exit", "type", "cd", "pwd", "export", "alias", "jobs", "fg", "bg", "source", ".",
    ] {
        if builtin.starts_with(prefix) {
            names.insert(builtin.to_string());
        }
    }

    // Binaries from $PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with(prefix) {
                            // Only include executable files
                            if let Ok(meta) = entry.metadata() {
                                if meta.is_file() {
                                    use std::os::unix::fs::PermissionsExt;
                                    if meta.permissions().mode() & 0o111 != 0 {
                                        names.insert(name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    names
        .into_iter()
        .map(|name| Pair {
            display: name.clone(),
            replacement: name,
        })
        .collect()
}

// ── Path completion (arguments) ───────────────────────────────────────────────

fn complete_path(prefix: &str) -> Vec<Pair> {
    // Expand tilde
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let expanded = if prefix == "~" {
        home.clone()
    } else if prefix.starts_with("~/") {
        prefix.replacen('~', &home, 1)
    } else {
        prefix.to_string()
    };

    // Split into directory and partial filename
    let (dir, file_prefix) = if expanded.contains('/') {
        let idx = expanded.rfind('/').unwrap();
        let d = if idx == 0 {
            "/".to_string()
        } else {
            expanded[..idx].to_string()
        };
        (d, expanded[idx + 1..].to_string())
    } else {
        (".".to_string(), expanded.clone())
    };

    let mut candidates = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.starts_with(&file_prefix))
            .collect();
        names.sort();

        for name in names {
            // Build the full replacement, preserving the original prefix style (tilde etc.)
            let full_path = if dir == "." {
                name.clone()
            } else if dir == "/" {
                format!("/{}", name)
            } else {
                format!("{}/{}", dir, name)
            };

            // Restore tilde if the user typed it
            let replacement = if prefix.starts_with("~/") || prefix == "~" {
                full_path.replacen(&home, "~", 1)
            } else {
                full_path.clone()
            };

            // Append '/' for directories so the user can keep tabbing into them
            let entry_path = std::path::Path::new(&full_path);
            let (display, replacement) = if entry_path.is_dir() {
                (format!("{}/", name), format!("{}/", replacement))
            } else {
                (name, replacement)
            };

            candidates.push(Pair {
                display,
                replacement,
            });
        }
    }

    candidates
}

// ── rustyline Helper trait (required boilerplate) ─────────────────────────────

impl Helper for ShellCompleter {}

impl Hinter for ShellCompleter {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        let enabled = std::env::var("RSH_HISTORY_HINTS").unwrap_or_else(|_| "1".to_string()) == "1";

        if enabled {
            self.hinter.hint(line, pos, ctx)
        } else {
            None
        }
    }
}

impl Highlighter for ShellCompleter {
    // Dim the history hint so it looks like fish's grey suggestion
    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        std::borrow::Cow::Owned(format!("\x1b[2m{}\x1b[0m", hint))
    }
}

impl Validator for ShellCompleter {}
