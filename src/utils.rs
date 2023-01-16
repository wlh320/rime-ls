use ropey::Rope;
use tower_lsp::lsp_types::Position;

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

/// int to sort_text string, with leading zero, e.g., 1 -> "z0001"
pub fn order_to_sort_text(order: usize, len: usize) -> String {
    // add a 'z' in the beginning
    format!("z{:0len$}", order, len = len)
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
