use std::env;
use std::ffi::CStr;

/// Parses standard Bash PS1 escape sequences into a real prompt string.
pub fn format_prompt(ps1: &str) -> String {
    let mut prompt = String::new();
    let mut chars = ps1.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next_c) = chars.peek() {
                match next_c {
                    // \u: Username
                    'u' => {
                        chars.next();
                        prompt.push_str(&env::var("USER").unwrap_or_else(|_| "user".to_string()));
                    }
                    // \A: Current time in 24-hour HH:MM format
                    'A' => {
                        chars.next();
                        unsafe {
                            let t = libc::time(std::ptr::null_mut());
                            let tm = libc::localtime(&t);
                            if !tm.is_null() {
                                prompt.push_str(&format!(
                                    "{:02}:{:02}",
                                    (*tm).tm_hour,
                                    (*tm).tm_min
                                ));
                            }
                        }
                    }
                    // \t: Current time in 24-hour HH:MM:SS format
                    't' => {
                        chars.next();
                        unsafe {
                            let t = libc::time(std::ptr::null_mut());
                            let tm = libc::localtime(&t);
                            if !tm.is_null() {
                                prompt.push_str(&format!(
                                    "{:02}:{:02}:{:02}",
                                    (*tm).tm_hour,
                                    (*tm).tm_min,
                                    (*tm).tm_sec
                                ));
                            }
                        }
                    }
                    // \h: Hostname (short)
                    'h' => {
                        chars.next();
                        let mut buf = [0u8; 256];
                        unsafe {
                            if libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len())
                                == 0
                            {
                                if let Ok(c_str) =
                                    CStr::from_ptr(buf.as_ptr() as *const libc::c_char).to_str()
                                {
                                    prompt.push_str(c_str.split('.').next().unwrap_or("localhost"));
                                }
                            }
                        }
                    }
                    // \w: Working directory (with ~ for $HOME)
                    'w' => {
                        chars.next();
                        if let Ok(cwd) = env::current_dir() {
                            let mut cwd_str = cwd.to_string_lossy().to_string();
                            if let Ok(home) = env::var("HOME") {
                                if cwd_str.starts_with(&home) {
                                    cwd_str = cwd_str.replacen(&home, "~", 1);
                                }
                            }
                            prompt.push_str(&cwd_str);
                        }
                    }
                    // \W: Basename of working directory
                    'W' => {
                        chars.next();
                        if let Ok(cwd) = env::current_dir() {
                            if let Some(name) = cwd.file_name() {
                                prompt.push_str(&name.to_string_lossy());
                            } else {
                                prompt.push('/');
                            }
                        }
                    }
                    // \$: '#' for root, '$' for regular users
                    '$' => {
                        chars.next();
                        unsafe {
                            if libc::geteuid() == 0 {
                                prompt.push('#');
                            } else {
                                prompt.push('$');
                            }
                        }
                    }
                    // \e: ASCII Escape character (for colors)
                    'e' => {
                        chars.next();
                        prompt.push('\x1b');
                    }
                    // \b: Custom rsh native Git branch detection!
                    'b' => {
                        chars.next();
                        if let Some(branch) = get_git_branch() {
                            // Format it however you like natively, e.g., "git:main"
                            prompt.push_str("git:");
                            prompt.push_str(&branch);
                        }
                    }
                    // \[ and \]: Non-printing character markers (CRITICAL for rustyline)
                    '[' => {
                        chars.next();
                        prompt.push('\x01'); // Readline SOH (Start of Header)
                    }
                    ']' => {
                        chars.next();
                        prompt.push('\x02'); // Readline STX (Start of Text)
                    }
                    // \\: Literal backslash
                    '\\' => {
                        chars.next();
                        prompt.push('\\');
                    }
                    // \n: Newline
                    'n' => {
                        chars.next();
                        prompt.push('\n');
                    }
                    _ => {
                        // Unrecognized escape, leave the backslash
                        prompt.push('\\');
                    }
                }
            } else {
                prompt.push('\\');
            }
        } else {
            prompt.push(c);
        }
    }
    prompt
}
fn get_git_branch() -> Option<String> {
    use std::process::{Command, Stdio};

    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null()) // Hide errors if not in a git repo
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Some(branch);
        }
    }
    None
}
