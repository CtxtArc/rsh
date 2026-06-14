# rsh (Rust Shell)

`rsh` is a custom Unix shell written from scratch in Rust. It features an AST-based parser, native job control, and several modern quality-of-life features designed to make terminal navigation and scripting faster and more ergonomic.

Instead of just wrapping external commands, `rsh` brings common developer tasks—like fuzzy path jumping, JSON parsing, and advanced text matching—directly into the shell's memory space to minimize process forking.

## Signature Features

* **The Anonymous Operator (`_`):** Built-in, blazing-fast fuzzy path resolution powered by `nucleo`. Instead of chaining `find` or `fzf`, just drop `_` into any path.
* *Example:* `cd ~/_/rsh` instantly resolves to `/home/user/coding/rust/rsh`.
* *Example:* `cat ./_/main` expands to `./src/main.rs`.


* **Extended "Super-Operators":** The standard `test` (`[`) command has been supercharged with native string, type, and file operators, completely avoiding the need for `grep` or complex regex hacks:
* **String Matching:** `-contains`, `-starts`, `-ends` (e.g., `[ "$VAR" -starts "prod_" ]`)
* **Type Checking:** `-isint`, `-isnum` (e.g., `[ -isint "$USER_INPUT" ]`)
* **File Inspection:** `-fcontains` (e.g., `[ config.env -fcontains "DEBUG=true" ]`)


* **Zero-Syscall JSON Cache:** A native `readjson` builtin that parses JSON files directly into the shell's internal state, allowing you to use structured config data as standard environment variables without expensive subshells.
* **Robust Job Control:** Built directly on top of `libc` to properly handle process group isolation, terminal handoffs (`tcsetpgrp`), and signals (`SIGTSTP`, `Ctrl+Z`, `fg`, `bg`).
* **Advanced Expansion:** Supports `$((...))` arithmetic evaluation via a custom Shunting-Yard engine, command substitution `$(cmd)`, and parameter expansion `${VAR:-default}`.

## Architecture

`rsh` parses scripts and REPL input into a recursive Abstract Syntax Tree (AST). This allows for clean, native handling of pipelines, logical operators (`&&`, `||`), and nested block statements (`if/else`, `while`, `for`).

### Project Structure

```text
src/
├── main.rs          # Entry point, REPL loop, and signal handling
├── state.rs         # Shell state, environment map, JSON cache, and job tables
├── types.rs         # Enums for AST nodes, Builtins, and Operators
├── tokenizer.rs     # Lexical analysis and quote/escape handling
├── parser.rs        # AST construction and syntax validation
├── executor.rs      # AST evaluation, process spawning, and pipeline routing
├── builtins.rs      # Native commands (cd, readjson, [, source, etc.)
├── expand.rs        # Variable substitution, math evaluation, and subshells
└── fuzzy.rs         # Headless nucleo/ignore engine for the `_` operator

```

## Installation

Ensure you have Rust and Cargo installed, then clone and build:

```bash
git clone https://github.com/YourUsername/rsh
cd rsh
cargo install --path .

```

## Quick Start

`rsh` can be used as your interactive daily driver or as a script interpreter.

**Interactive usage:**

```bash
# Jump directly to a deeply nested project folder
rsh$ cd ~/_/rsh

# Check if an input is a valid number
rsh$ if [ -isnum "$INPUT" ]; then echo "Valid math"; fi

```

**Running scripts:**

```bash
# Execute a script file
rsh ./setup.rsh

# Execute an inline command
rsh -c 'for X in 1 2 3 ; do echo $((X * 10)) ; done'

```

## Why rsh?

`rsh` started as an exercise in systems programming and grew into a daily-driver shell. While tools like `bash` and `zsh` are incredibly powerful, they often rely on spawning external binaries (`grep`, `jq`, `find`) for common data tasks. By bringing structured data parsing, type-checking, and fuzzy path resolution natively into the shell, `rsh` aims to provide a faster, more modern command-line experience out of the box.

---

How does that look? It highlights exactly why someone would want to write a script in `rsh` instead of Bash.
