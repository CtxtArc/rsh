# rsh Development Todo List

## UX & Polish
[ ] Implement TTY-aware output for ANSI color support (ls/grep compatibility)
[ ] Add Tab-completion for binaries and file paths using rustyline
[ ] Add dynamic PS1 prompt (Git branch awareness)

## Advanced I/O
[ ] Support stream merging (2>&1 redirection)
[ ] Implement here-docs (<< EOF) for multi-line string input

## add builtins
[ ] source builtin to reload a file
[ ] Native `readjson` builtin (Parse JSON directly into shell variables)
