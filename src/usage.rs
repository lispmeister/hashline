use std::fs::{create_dir_all, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
pub enum UsageResult {
    Success,
    Mismatch,
    Error,
}

#[derive(Clone, Copy, Debug)]
pub struct UsageEvent<'a> {
    pub command: &'a str,
    pub result: UsageResult,
    pub emit_updated: bool,
    pub used_input_file: bool,
}

pub fn log_event(event: UsageEvent<'_>) -> io::Result<()> {
    if std::env::var_os("HASHLINE_DISABLE_USAGE_LOG").is_some() {
        return Ok(());
    }

    let path = usage_log_path();
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let result = match event.result {
        UsageResult::Success => "success",
        UsageResult::Mismatch => "mismatch",
        UsageResult::Error => "error",
    };

    writeln!(
        file,
        "{timestamp},{},{},{},{}",
        event.command,
        result,
        if event.emit_updated { 1 } else { 0 },
        if event.used_input_file { 1 } else { 0 },
    )
}

fn usage_log_path() -> PathBuf {
    if let Some(custom) = std::env::var_os("HASHLINE_USAGE_LOG") {
        return PathBuf::from(custom);
    }
    default_usage_path()
}

#[cfg(windows)]
fn default_usage_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        PathBuf::from(appdata).join("hashline").join("usage.log")
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("hashline-usage.log")
    }
}

#[cfg(not(windows))]
fn default_usage_path() -> PathBuf {
    if let Some(xdg_state) = std::env::var_os("XDG_STATE_HOME") {
        PathBuf::from(xdg_state).join("hashline").join("usage.log")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".local/state")
            .join("hashline")
            .join("usage.log")
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("hashline-usage.log")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use tempfile::NamedTempFile;

    fn clear_env() {
        std::env::remove_var("HASHLINE_USAGE_LOG");
        std::env::remove_var("HASHLINE_DISABLE_USAGE_LOG");
    }
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn log_event_writes_to_custom_path() {
        let _guard = env_lock().lock().unwrap();
        clear_env();

        let temp = NamedTempFile::new().unwrap();
        let temp_path = temp.into_temp_path();
        let log_path = temp_path.to_path_buf();

        std::env::set_var("HASHLINE_USAGE_LOG", &log_path);

        log_event(UsageEvent {
            command: "apply",
            result: UsageResult::Success,
            emit_updated: true,
            used_input_file: true,
        })
        .unwrap();
        log_event(UsageEvent {
            command: "apply",
            result: UsageResult::Mismatch,
            emit_updated: false,
            used_input_file: false,
        })
        .unwrap();

        std::env::remove_var("HASHLINE_USAGE_LOG");
        let content = fs::read_to_string(&log_path).unwrap();
        assert_eq!(content.lines().count(), 2);
        assert!(content.contains("apply"));
        assert!(content.contains("mismatch"));
        temp_path.close().unwrap();
        clear_env();
    }

    #[test]
    fn disable_env_prevents_logging() {
        let _guard = env_lock().lock().unwrap();
        clear_env();

        let temp = NamedTempFile::new().unwrap();
        let temp_path = temp.into_temp_path();
        let log_path = temp_path.to_path_buf();

        std::env::set_var("HASHLINE_USAGE_LOG", &log_path);
        std::env::set_var("HASHLINE_DISABLE_USAGE_LOG", "1");

        log_event(UsageEvent {
            command: "apply",
            result: UsageResult::Error,
            emit_updated: false,
            used_input_file: false,
        })
        .unwrap();

        let metadata = fs::metadata(&log_path).unwrap();
        assert_eq!(metadata.len(), 0);
        temp_path.close().unwrap();
        clear_env();
    }
}
