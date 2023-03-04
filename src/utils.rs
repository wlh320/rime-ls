use ropey::Rope;
use std::path::{Path, PathBuf};
use tower_lsp::lsp_types::Position;

use crate::consts::AUTO_TRIGGER_RE;

/// UTF-16 Position -> UTF-8 offset
pub fn position_to_offset(rope: &Rope, position: Position) -> Option<usize> {
    let (line, col) = (position.line as usize, position.character as usize);
    // position is at the end of rope
    if line == rope.len_lines() && col == 0 {
        return Some(rope.len_chars());
    }
    (line < rope.len_lines()).then_some(line).and_then(|line| {
        let col8 = rope.line(line).try_utf16_cu_to_char(col).ok()?;
        let offset = rope.try_line_to_char(line).ok()? + col8;
        Some(offset)
    })
}

/// UTF-8 offset -> UTF-16 Position
pub fn offset_to_position(rope: &Rope, offset: usize) -> Option<Position> {
    let line = rope.try_char_to_line(offset).ok()?;
    let col8 = offset - rope.try_line_to_char(line).ok()?;
    (line < rope.len_lines()).then_some(line).and_then(|line| {
        let col16 = rope.line(line).try_char_to_utf16_cu(col8).ok()?;
        Some(Position::new(line as u32, col16 as u32))
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
