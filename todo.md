# rsh Development Todo List

## Phase 1: UX & Polish
[ ] Implement TTY-aware output for ANSI color support (ls/grep compatibility)
[ ] Add Tab-completion for binaries and file paths using rustyline
[ ] Add dynamic PS1 prompt (Git branch awareness)

## Phase 2: Advanced I/O
[ ] Support stream merging (2>&1 redirection)
[ ] Implement here-docs (<< EOF) for multi-line string input

## Phase 3: The "Superpower"
[ ] Native `readjson` builtin (Parse JSON directly into shell variables)

## add builtins
[ ] source builtin to reload a file
