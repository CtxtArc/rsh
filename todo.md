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
[x] Implement here-docs (<< EOF) for multi-line string input

## add builtins
[x] source builtin to reload a file
[x] Native `readjson` builtin (Parse JSON directly into shell variables)
[x] make readjson way faster by caching vars
[x] time command

## expand scripting
[x] Native operators (eg. -f -ne -z ..)
[x] operators -contains, -starts, -ends ...
[ ] lists and arrays

## anonymous operator
[x] anonymous operator (eg. cd ~/coding/\_/rsh/ will resolve to ~/coding/rust/rsh/ because it will find the missing folder)
[x] make cd resolve recursively using `_` (eg. cd ~/\_/rsh/ will resolve all dirs bewteen ~/ and rsh/ and find the path: ~/coding/rust/rsh/)
