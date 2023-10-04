use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize)]
pub struct Config {
    pub port: Option<u16>,
}

#[derive(Deserialize)]
pub struct RunRequest {
    pub command: String,
}

#[derive(Serialize)]
pub struct RunResponse {
    pub success: bool,
    pub pid: Option<u32>,
}

pub const MUTEX_NAME: &str = "Global\\DarktideLocalServerMutex";
pub const DEFAULT_PORT: u16 = 41012;
pub const CONFIG_NAME: &str = "config.json";
pub const SUCCESS: &str = "success";
pub const PID: &str = "pid";