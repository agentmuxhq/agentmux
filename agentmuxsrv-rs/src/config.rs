use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "agentmuxsrv-rs", about = "AgentMux Rust backend server")]
pub struct CliArgs {
    /// Path to wave data directory (overrides WAVETERM_DATA_HOME)
    #[arg(long = "wavedata")]
    pub wavedata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub auth_key: String,
    pub data_home: String,
    pub config_home: String,
    pub app_path: String,
    pub is_dev: bool,
    pub version: &'static str,
    pub build_time: &'static str,
}

impl Config {
    /// Build config from env vars + CLI args.
    /// Removes WAVETERM_AUTH_KEY from the environment after reading (matching Go behavior).
    pub fn from_env_and_args(args: &CliArgs) -> Result<Self, String> {
        let auth_key = std::env::var("WAVETERM_AUTH_KEY")
            .map_err(|_| "WAVETERM_AUTH_KEY environment variable is required".to_string())?;

        if auth_key.is_empty() {
            return Err("WAVETERM_AUTH_KEY must not be empty".to_string());
        }

        // Remove from env after read (matching Go authkey.go:50)
        std::env::remove_var("WAVETERM_AUTH_KEY");

        let data_home = args
            .wavedata
            .clone()
            .or_else(|| std::env::var("WAVETERM_DATA_HOME").ok())
            .unwrap_or_default();

        let config_home = std::env::var("WAVETERM_CONFIG_HOME").unwrap_or_default();
        let app_path = std::env::var("WAVETERM_APP_PATH").unwrap_or_default();
        let is_dev = std::env::var("WAVETERM_DEV")
            .map(|v| !v.is_empty() && v != "0")
            .unwrap_or(false);

        Ok(Config {
            auth_key,
            data_home,
            config_home,
            app_path,
            is_dev,
            version: env!("CARGO_PKG_VERSION"),
            build_time: option_env!("BUILD_TIME").unwrap_or("dev"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize config tests — they mutate process-global env vars
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn missing_auth_key_errors() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::remove_var("WAVETERM_AUTH_KEY");
        let args = CliArgs { wavedata: None };
        let result = Config::from_env_and_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("WAVETERM_AUTH_KEY"));
    }

    #[test]
    fn empty_auth_key_errors() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("WAVETERM_AUTH_KEY", "");
        let args = CliArgs { wavedata: None };
        let result = Config::from_env_and_args(&args);
        assert!(result.is_err());
        std::env::remove_var("WAVETERM_AUTH_KEY");
    }

    #[test]
    fn cli_wavedata_overrides_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("WAVETERM_AUTH_KEY", "test-key-12345");
        std::env::set_var("WAVETERM_DATA_HOME", "/from/env");
        let args = CliArgs {
            wavedata: Some("/from/cli".to_string()),
        };
        let config = Config::from_env_and_args(&args).unwrap();
        assert_eq!(config.data_home, "/from/cli");
        assert!(std::env::var("WAVETERM_AUTH_KEY").is_err());
        std::env::remove_var("WAVETERM_DATA_HOME");
    }

    #[test]
    fn env_var_parsing() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("WAVETERM_AUTH_KEY", "test-key-67890");
        std::env::set_var("WAVETERM_DATA_HOME", "/data");
        std::env::set_var("WAVETERM_CONFIG_HOME", "/config");
        std::env::set_var("WAVETERM_APP_PATH", "/app");
        std::env::set_var("WAVETERM_DEV", "1");
        let args = CliArgs { wavedata: None };
        let config = Config::from_env_and_args(&args).unwrap();
        assert_eq!(config.data_home, "/data");
        assert_eq!(config.config_home, "/config");
        assert_eq!(config.app_path, "/app");
        assert!(config.is_dev);
        std::env::remove_var("WAVETERM_DATA_HOME");
        std::env::remove_var("WAVETERM_CONFIG_HOME");
        std::env::remove_var("WAVETERM_APP_PATH");
        std::env::remove_var("WAVETERM_DEV");
    }
}
