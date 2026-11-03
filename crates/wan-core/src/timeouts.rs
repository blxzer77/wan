use std::time::Duration;

pub const SIGTERM_GRACE: Duration = Duration::from_secs(5);
pub const MCP_CANCEL: Duration = Duration::from_secs(2);
pub const ACP_CANCEL: Duration = Duration::from_secs(10);
pub const MODEL_RESPONSE_HEADER: Duration = Duration::from_secs(10 * 60);
pub const MODEL_STREAM_IDLE: Duration = Duration::from_secs(5 * 60);
pub const COMMAND_FOREGROUND_DEFAULT: Duration = Duration::from_secs(2 * 60);
pub const COMMAND_FOREGROUND_MAX: Duration = Duration::from_secs(10 * 60);
pub const COMMAND_BACKGROUND: Duration = Duration::from_secs(2 * 60 * 60);
pub const MCP_STARTUP: Duration = Duration::from_secs(30);
pub const MCP_CALL_DEFAULT: Duration = Duration::from_secs(2 * 60);
pub const MCP_CALL_MAX: Duration = Duration::from_secs(30 * 60);
pub const HELPER_IDLE: Duration = Duration::from_secs(10 * 60);
pub const TURN: Duration = Duration::from_secs(2 * 60 * 60);
pub const ROUTINE: Duration = Duration::from_secs(30 * 60);
pub const BACKGROUND_APPROVAL: Duration = Duration::from_secs(30 * 60);
