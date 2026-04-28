use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchingState {
    pub id: u64,
    pub current_step: usize,
    pub instance_id: String,
    pub version: String,
    pub java_path: Option<String>,
    pub java_major_version: Option<i32>,
    pub game_directory: String,
    pub pid: u32,
    pub full_command: String,
    pub start_time: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LaunchStep {
    Idle = 0,
    SelectJre = 1,
    ValidateFiles = 2,
    LaunchGame = 3,
    Running = 4,
    Crashed = 5,
    Exited = 6,
}

impl From<usize> for LaunchStep {
    fn from(v: usize) -> Self {
        match v {
            1 => LaunchStep::SelectJre,
            2 => LaunchStep::ValidateFiles,
            3 => LaunchStep::LaunchGame,
            4 => LaunchStep::Running,
            5 => LaunchStep::Crashed,
            6 => LaunchStep::Exited,
            _ => LaunchStep::Idle,
        }
    }
}
