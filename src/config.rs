use serde::{Serialize, Deserialize};
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// rime share data dir
    pub shared_data_dir: PathBuf,
    /// rime user data dir
    pub user_data_dir: PathBuf,
    /// rime log data dir
    pub log_dir: PathBuf,
    /// max number of candidates
    pub max_candidates: usize,
    /// if some, only trigger completion with special keys
    pub trigger_characters: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            shared_data_dir: default_shared_data_dir(),
            user_data_dir: default_user_data_dir(),
            log_dir: default_log_dir(),
            max_candidates: DEFAULT_MAX_CANDIDATES,
            trigger_characters: default_trigger_characters()
        }
    }
}

const DEFAULT_MAX_CANDIDATES: usize = 10;

fn default_trigger_characters() -> Option<Vec<String>> {
    Some(vec![String::from(">")])
}

fn default_shared_data_dir() -> PathBuf {
    PathBuf::from("/usr/share/rime-data")
}

fn default_user_data_dir() -> PathBuf {
    let proj_dirs = ProjectDirs::from("com", "rimels",  "Rime-Ls").unwrap();
    proj_dirs.data_dir().to_path_buf()
}

fn default_log_dir() -> PathBuf {
    let proj_dirs = ProjectDirs::from("com", "rimels",  "Rime-Ls").unwrap();
    proj_dirs.cache_dir().to_path_buf()
}


#[test]
fn test_default_config() {
    let config: Config = Default::default();
    assert_eq!(config.shared_data_dir, default_shared_data_dir());
    assert_eq!(config.user_data_dir, default_user_data_dir());
    assert_eq!(config.log_dir, default_log_dir());
    assert_eq!(config.max_candidates, DEFAULT_MAX_CANDIDATES);
    assert_eq!(config.trigger_characters, default_trigger_characters());
}

