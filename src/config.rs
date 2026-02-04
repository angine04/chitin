use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub provider: ProviderConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            provider: ProviderConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_socket_path")]
    pub socket_path: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
        }
    }
}

fn default_socket_path() -> String {
    "/tmp/chitin.sock".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProviderConfig {
    #[serde(default = "default_provider_type")]
    pub type_: String,
    #[serde(default)]
    pub openai: OpenAiConfig,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            type_: default_provider_type(),
            openai: OpenAiConfig::default(),
        }
    }
}

fn default_provider_type() -> String {
    "openai".to_string()
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct OpenAiConfig {
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        Self::load_reload().unwrap_or_else(|e| {
            eprintln!("Info: using default config or env vars (reason: {})", e);
            let mut config = Config::default();
            merge_env_vars(&mut config);
            config
        })
    }

    pub fn load_reload() -> Result<Self, String> {
        let config_path = get_config_path();

        let mut config = if let Some(path) = &config_path {
            if path.exists() {
                match fs::read_to_string(path) {
                    Ok(content) => toml::from_str(&content).map_err(|e| e.to_string())?,
                    Err(e) => return Err(e.to_string()),
                }
            } else {
                Config::default()
            }
        } else {
            Config::default()
        };

        // Environment variables override config file
        merge_env_vars(&mut config);
        Ok(config)
    }
}

fn get_config_path() -> Option<PathBuf> {
    // 1. Environment variable
    if let Ok(path) = env::var("CHITIN_CONFIG") {
        return Some(PathBuf::from(path));
    }

    // 2. Current directory
    let current_dir_config = PathBuf::from("chitin.toml");
    if current_dir_config.exists() {
        return Some(current_dir_config);
    }

    // 3. ~/.config/chitin/config.toml (Explicit XDG-style support)
    if let Some(base_dirs) = directories::BaseDirs::new() {
        let xdg_config = base_dirs
            .home_dir()
            .join(".config")
            .join("chitin")
            .join("config.toml");
        if xdg_config.exists() {
            return Some(xdg_config);
        }
    }

    // 4. System default (Platform specific)
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "user", "chitin") {
        let sys_config = proj_dirs.config_dir().join("config.toml");
        if sys_config.exists() {
            return Some(sys_config);
        }
    }

    None
}

fn merge_env_vars(config: &mut Config) {
    if let Ok(val) = env::var("CHITIN_SOCKET_PATH") {
        config.server.socket_path = val;
    }
    if let Ok(val) = env::var("CHITIN_PROVIDER") {
        config.provider.type_ = val;
    }
    if let Ok(val) = env::var("CHITIN_API_BASE") {
        config.provider.openai.api_base = Some(val);
    }
    if let Ok(val) = env::var("CHITIN_API_KEY") {
        config.provider.openai.api_key = Some(val);
    }
    if let Ok(val) = env::var("CHITIN_MODEL") {
        config.provider.openai.model = Some(val);
    }
}
