use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// all configs of rime-ls
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// if enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// rime share data dir
    #[serde(default = "default_shared_data_dir")]
    pub shared_data_dir: PathBuf,
    /// rime user data dir
    #[serde(default = "default_user_data_dir")]
    pub user_data_dir: PathBuf,
    /// rime log data dir
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,
    /// max number of candidates
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,
    /// if not empty, only trigger completion with special keys
    #[serde(default = "default_trigger_characters")]
    pub trigger_characters: Vec<String>,
}

/// settings that can be tweaked during running
#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    /// enabled
    pub enabled: Option<bool>,
    /// max number of candidates
    pub max_candidates: Option<usize>,
    /// if not empty, only trigger completion with special keys
    pub trigger_characters: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: default_enabled(),
            shared_data_dir: default_shared_data_dir(),
            user_data_dir: default_user_data_dir(),
            log_dir: default_log_dir(),
            max_candidates: default_max_candidates(),
            trigger_characters: default_trigger_characters(),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_max_candidates() -> usize {
    10
}

fn default_trigger_characters() -> Vec<String> {
    vec![]
}

fn default_shared_data_dir() -> PathBuf {
    PathBuf::from("/usr/share/rime-data")
}

fn default_user_data_dir() -> PathBuf {
    let proj_dirs = ProjectDirs::from("com", "rimels", "Rime-Ls").unwrap();
    proj_dirs.data_dir().to_path_buf()
}

fn default_log_dir() -> PathBuf {
    let proj_dirs = ProjectDirs::from("com", "rimels", "Rime-Ls").unwrap();
    proj_dirs.cache_dir().to_path_buf()
}

#[test]
fn test_default_config() {
    let config: Config = Default::default();
    assert_eq!(config.enabled, default_enabled());
    assert_eq!(config.shared_data_dir, default_shared_data_dir());
    assert_eq!(config.user_data_dir, default_user_data_dir());
    assert_eq!(config.log_dir, default_log_dir());
    assert_eq!(config.max_candidates, default_max_candidates());
    assert_eq!(config.trigger_characters, default_trigger_characters());
}
