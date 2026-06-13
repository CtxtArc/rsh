# rsh Development Todo List


## UX & Polish
[x] proper escaping in tokenize
[x] Implement TTY-aware output for ANSI color support (ls/grep compatibility)
[x] Add Tab-completion for binaries and file paths using rustyline
[x] Add Tab-completion for commands from history 
[x] Add dynamic PS1 and PS2 prompt 
[x] add Git branch awareness for PS1

## Advanced I/O
[x] Support stream merging (2>&1 redirection)
[ ] Implement here-docs (<< EOF) for multi-line string input

## add builtins
[x] source builtin to reload a file
[ ] Native `readjson` builtin (Parse JSON directly into shell variables)
[ ] Native operators (eg. -f -ne -z ..)
