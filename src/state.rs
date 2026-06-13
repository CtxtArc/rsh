use crate::types::ASTNode;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum JobStatus {
    Running,
    Stopped,
}

pub struct Job {
    pub id: usize,
    pub pgid: i32,
    pub command: String,
    pub status: JobStatus,
}

pub struct ShellState {
    pub aliases: HashMap<String, String>,
    pub functions: HashMap<String, ASTNode>,
    pub jobs: Vec<Job>,
    pub job_id_counter: usize,
    pub last_exit_status: i32,
}

impl ShellState {
    pub fn new() -> Self {
        ShellState {
            aliases: HashMap::new(),
            functions: HashMap::new(),
            jobs: Vec::new(),
            job_id_counter: 1,
            last_exit_status: 0,
        }
    }
}
