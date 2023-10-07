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
    /// if not empty, these characters will trigger completion for paging
    #[serde(default = "default_paging_characters")]
    pub paging_characters: Vec<String>,
    /// if not empty, only trigger completion with special keys
    #[serde(default = "default_trigger_characters")]
    pub trigger_characters: Vec<String>,
    /// if set, completion request with this string will trigger「方案選單」
    #[serde(default = "default_schema_trigger_character")]
    pub schema_trigger_character: String,
    /// if set, when a delete action arrives the number of max tokens, emit a force new_typing
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// if CompletionItem is always incomplete
    #[serde(default = "default_always_incomplete")]
    pub always_incomplete: bool,
    /// if preselect first CompletionItem
    #[serde(default = "default_preselect_first")]
    pub preselect_first: bool,
}

/// settings that can be tweaked during running
#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    /// enabled
    pub enabled: Option<bool>,
    /// max number of candidates
    pub max_candidates: Option<usize>,
    /// if not empty, these characters will trigger completion for paging
    pub paging_characters: Option<Vec<String>>,
    /// if not empty, only trigger completion with special keys
    pub trigger_characters: Option<Vec<String>>,
    /// if set, completion request with this string will trigger「方案選單」
    pub schema_trigger_character: Option<String>,
    /// if set, when a delete action arrives the number of max tokens, emit a force new_typing
    pub max_tokens: Option<usize>,
    /// if CompletionItem is always incomplete
    pub always_incomplete: Option<bool>,
    /// if preselect first CompletionItem
    pub preselect_first: Option<bool>,
}

macro_rules! apply_setting {
    ($to:ident <- $from:ident.$field:ident) => {
        if let Some(v) = $from.$field {
            $to.$field = v;
        }
    };
    ($to:ident <- $from:ident.$field:ident, |$v:ident| $b:block) => {
        if let Some($v) = $from.$field {
            $b
            $to.$field = $v;
        }
    };
}
pub(crate) use apply_setting;

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: default_enabled(),
            shared_data_dir: default_shared_data_dir(),
            user_data_dir: default_user_data_dir(),
            log_dir: default_log_dir(),
            max_candidates: default_max_candidates(),
            paging_characters: default_paging_characters(),
            trigger_characters: default_trigger_characters(),
            schema_trigger_character: default_schema_trigger_character(),
            max_tokens: default_max_tokens(),
            always_incomplete: default_always_incomplete(),
            preselect_first: default_preselect_first(),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_always_incomplete() -> bool {
    false
}

fn default_max_candidates() -> usize {
    10
}

fn default_max_tokens() -> usize {
    0
}

fn default_trigger_characters() -> Vec<String> {
    Vec::default()
}

fn default_paging_characters() -> Vec<String> {
    ["-", "=", ",", "."].map(|x| x.to_string()).to_vec()
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

fn default_schema_trigger_character() -> String {
    String::default()
}

fn default_preselect_first() -> bool {
    false
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
    assert_eq!(
        config.schema_trigger_character,
        default_schema_trigger_character()
    );
    assert_eq!(config.always_incomplete, default_always_incomplete());
    assert_eq!(config.max_tokens, default_max_tokens());
}

#[test]
fn test_apply_settings() {
    let mut config: Config = Default::default();
    let settings: Settings = Settings {
        enabled: Some(false),
        max_candidates: Some(100),
        paging_characters: Some(vec![",".to_string(), ".".to_string()]),
        trigger_characters: Some(vec!["foo".to_string()]),
        schema_trigger_character: Some(String::from("bar")),
        max_tokens: None,
        always_incomplete: None,
        preselect_first: None,
    };
    // apply settings with macro
    let mut test_val = vec!["baz".to_string()];
    apply_setting!(config <- settings.enabled);
    apply_setting!(config <- settings.max_candidates);
    apply_setting!(config <- settings.paging_characters);
    apply_setting!(config <- settings.trigger_characters, |v| {
        test_val = v.clone();
    });
    apply_setting!(config <- settings.schema_trigger_character);
    // verify
    assert_eq!(config.enabled, false);
    assert_eq!(config.max_candidates, 100);
    assert_eq!(
        config.paging_characters,
        vec![",".to_string(), ".".to_string()]
    );
    assert_eq!(config.trigger_characters, vec!["foo".to_string()]);
    assert_eq!(config.schema_trigger_character, String::from("bar"));
    assert_eq!(test_val, vec!["foo".to_string()]);
}
