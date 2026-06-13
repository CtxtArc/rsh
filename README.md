# rsh (Rust Shell)

`rsh` is a custom, high-performance Unix shell written from scratch in Rust. It implements a fully Turing-complete scripting environment, complete with a recursive Abstract Syntax Tree (AST) compiler, native job control, and custom mathematical and regex evaluation engines.

## Architecture

Unlike simple command-wrappers, `rsh` is a core systems utility that interacts directly with the Linux kernel via `libc`.

### Key Components

* **Recursive AST Compiler:** `rsh` does not execute commands line-by-line. It parses scripts into a recursive tree structure, allowing for infinite nesting of `if/else`, `for`, and `while` logic.
* **Shunting-Yard Math Engine:** Includes a custom mathematical compiler implementing Dijkstra’s Shunting-Yard algorithm to evaluate `$((...))` arithmetic with full operator precedence support.
* **Lazy Expansion Engine:** A custom FSM (Finite State Machine) that handles subshells (`$(...)`), parameter expansion (`${VAR:-default}`), and native regex matching (`=~`) with deferred evaluation.
* **Process Group Isolation:** Implements robust Job Control using `tcsetpgrp` and `waitpid`, allowing the shell to manage background processes (`&`) and safely intercept `SIGTSTP` (`Ctrl-Z`) signals without terminating the parent shell.

## Features

* **Turing-Complete:** Supports `if/then/else`, `for`, and `while` loops.
* **Native Arithmetic:** Native `$((...))` evaluation for complex math expressions.
* **Regex Support:** Built-in `=~` operator for high-performance text validation without spawning sub-processes.
* **I/O Routing:** Full support for standard pipes (`|`) and multi-stream redirection (`>`, `>>`, `2>`, `2>>`).
* **Function Support:** Define and override functions in-memory with positional argument support (`$1`, `$2`, etc.).

## Installation

```bash
# Clone the repository
git clone https://github.com/CtxtArc/rsh
cd rsh

# Install globally to ~/.cargo/bin
cargo install --path .

```

## Usage

`rsh` functions as both an interactive REPL and a scripting language interpreter. You can run scripts by providing a file path:

```bash
# Run a script directly
./your_script.rsh

# Or run a command string
rsh -c "for X in 1 2 3 ; do echo $X ; done"

```

## Why rsh?

Most shells delegate complex text processing to external binaries (`grep`, `sed`, `awk`). `rsh` is designed for speed and modularity—by keeping these features native to the shell process, it avoids the overhead of context switching and process forking, making it exceptionally fast for complex automation tasks.


src/
├── main.rs          # Entry point, REPL loop
├── state.rs         # ShellState, Job, JobStatus
├── types.rs         # Command, ASTNode, Operator, LogicalGroup, Builtin
├── tokenizer.rs     # tokenize(), is_incomplete()
├── parser.rs        # parse_ast(), parse_logic(), parse_pipeline_from_tokens(), split_statements()
├── executor.rs      # evaluate_ast(), evaluate_tokens(), execute_single(), execute_pipeline()
├── builtins.rs      # Builtin::parse() + all builtin implementations
└── expand.rs        # expand_word(), expand_glob(), eval_math(), match_pattern(), find_in_path()
