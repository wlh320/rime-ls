use ropey::Rope;
use std::path::{Path, PathBuf};
use tower_lsp::lsp_types::{Position, PositionEncodingKind};

use crate::consts::AUTO_TRIGGER_RE;

#[derive(Default, Debug, Clone, Copy)]
pub enum Encoding {
    UTF8,
    #[default]
    UTF16,
    UTF32,
}

impl Encoding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Encoding::UTF8 => "utf-8",
            Encoding::UTF16 => "utf-16",
            Encoding::UTF32 => "utf-32",
        }
    }
}

pub fn select_encoding(options: Option<Vec<PositionEncodingKind>>) -> Encoding {
    match options {
        // we prefer utf-32 here because there are no conversion costs
        Some(v) if v.contains(&PositionEncodingKind::new("utf-32")) => Encoding::UTF32,
        Some(v) if v.contains(&PositionEncodingKind::new("utf-8")) => Encoding::UTF8,
        _ => Encoding::default(),
    }
}

/// UTF-8/16/32 Position -> char index
pub fn position_to_offset(rope: &Rope, position: Position, encoding: Encoding) -> Option<usize> {
    let (line, col) = (position.line as usize, position.character as usize);
    // position is at the end of rope
    if line == rope.len_lines() && col == 0 {
        return Some(rope.len_chars());
    }
    (line < rope.len_lines()).then_some(line).and_then(|line| {
        let col_offset = match encoding {
            Encoding::UTF8 => rope.line(line).try_byte_to_char(col).ok()?,
            Encoding::UTF16 => rope.line(line).try_utf16_cu_to_char(col).ok()?,
            Encoding::UTF32 => col,
        };
        //let col8 = rope.line(line).try_utf16_cu_to_char(col).ok()?;
        let offset = rope.try_line_to_char(line).ok()? + col_offset;
        Some(offset)
    })
}

/// char index -> UTF-8/16/32 Position
pub fn offset_to_position(rope: &Rope, offset: usize, encoding: Encoding) -> Option<Position> {
    let line = rope.try_char_to_line(offset).ok()?;
    let col_offset = offset - rope.try_line_to_char(line).ok()?;
    (line < rope.len_lines()).then_some(line).and_then(|line| {
        let col = match encoding {
            Encoding::UTF8 => rope.line(line).try_char_to_byte(col_offset).ok()?,
            Encoding::UTF16 => rope.line(line).try_char_to_utf16_cu(col_offset).ok()?,
            Encoding::UTF32 => col_offset,
        };
        Some(Position::new(line as u32, col as u32))
    })
}

pub enum DiffResult<'a> {
    Same,
    Add(&'a str),
    Delete(&'a str),
    New,
}

pub fn diff<'s>(old_text: &'s str, new_text: &'s str) -> DiffResult<'s> {
    if old_text == new_text {
        DiffResult::Same
    } else if let Some(suffix) = new_text.strip_prefix(old_text) {
        DiffResult::Add(suffix)
    } else if let Some(suffix) = old_text.strip_prefix(new_text) {
        DiffResult::Delete(suffix)
    } else {
        DiffResult::New
    }
}

/// int to sort_text string, with leading zero, e.g., 1 -> "z0001"
#[inline]
pub fn build_order_to_sort_text(max_candidates: usize) -> impl Fn(usize) -> String {
    let len = std::iter::successors(Some(max_candidates), |&n| (n >= 10).then_some(n / 10)).count();
    move |n| format!("z{n:0len$}")
}

/// return if we need to check the existence of trigger character
#[inline]
pub fn need_to_check_trigger(has_trigger: bool, line: &str) -> bool {
    has_trigger && !AUTO_TRIGGER_RE.is_match(line)
}

/// convert empty string to None
#[inline]
pub fn option_string(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// expand tilde in path, panics when home dir does not exist
pub fn expand_tilde(path: impl AsRef<Path>) -> PathBuf {
    if !path.as_ref().starts_with("~") {
        return path.as_ref().into();
    }
    let base_dirs = directories::BaseDirs::new().unwrap();
    let home_dir = base_dirs.home_dir();
    home_dir.join(path.as_ref().strip_prefix("~").unwrap())
}

/// rime's default shared data dir
pub fn rime_default_shared_data_dir() -> &'static str {
    // read environment variable first
    if let Some(var) = option_env!("RIME_DATA_DIR") {
        var
    } else if cfg!(target_os = "macos") {
        "/Library/Input Methods/Squirrel.app/Contents/SharedSupport"
    } else {
        // cannot determine shared data dir on windows
        // Ref: https://github.com/rime/home/wiki/SharedData
        "/usr/share/rime-data"
    }
}

#[inline]
fn char_is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

pub fn surrounding_word(s: &str) -> String {
    let end = s.len();
    let mut start = end;
    for ch in s.chars().rev() {
        if char_is_word(ch) {
            start -= ch.len_utf8();
        } else {
            break;
        }
    }
    s[start..end].to_string()
}

#[test]
fn test_surrounding_word() {
    assert_eq!(surrounding_word(""), "".to_string());
    assert_eq!(surrounding_word(" "), "".to_string());
    assert_eq!(surrounding_word("hello_world"), "hello_world".to_string());
    assert_eq!(surrounding_word("hello world"), "world".to_string());
    assert_eq!(surrounding_word("汉字nihao"), "汉字nihao".to_string());
    assert_eq!(surrounding_word("汉，字nihao"), "字nihao".to_string());
    assert_eq!(surrounding_word("汉。字nihao"), "字nihao".to_string());
}
