use regex::Regex;
use ropey::Rope;
use tower_lsp::lsp_types::Position;

pub fn get_pinyin(pre_text: &str) -> Option<String> {
    if pre_text.is_empty() {
        return None;
    }
    let regex = Regex::new(r"(?P<pinyin>[a-zA-Z]+)$").unwrap();
    if let Some(m) = regex.captures(pre_text) {
        return Some(m["pinyin"].to_string());
    }
    None
}

pub fn position_to_offset(rope: &Rope, position: &Position) -> Option<usize> {
    let char = rope.try_line_to_char(position.line as usize).ok()?;
    let offset = char + position.character as usize;
    Some(offset)
}

#[allow(dead_code)]
pub fn offset_to_position(rope: &Rope, offset: usize) -> Option<Position> {
    let line = rope.try_char_to_line(offset).ok()?;
    let first_char = rope.try_line_to_char(line).ok()?;
    let column = offset - first_char;
    Some(Position::new(line as u32, column as u32))
}
