use crate::rime::Rime;
use crate::utils::{diff, DiffResult};
use regex::Regex;

/// regex raw input to groups
#[derive(Debug)]
pub struct Input<'s> {
    pub raw_text: &'s str,
    pub trigger: Option<&'s str>,
    pub pinyin: &'s str,
    pub oper: &'s str,
    pub select: &'s str,
}

pub const PTN: &str = r"((?P<py>[a-zA-Z]+)(?P<op>[-=]*)(?P<se>[0-9]?))$";

// hack "format argument must be a string literal"
macro_rules! trg_ptn {
    () => {
        r"((?P<tr>[{}])(?P<py>[a-zA-Z]+)(?P<op>[-=]*)(?P<se>[0-9]?))$"
    };
}
pub(crate) use trg_ptn;

impl<'s> Input<'s> {
    pub fn from_str(re: &Regex, text: &'s str) -> Option<Self> {
        re.captures(text).map(|caps| Input {
            raw_text: caps.get(0).unwrap().as_str(),
            trigger: caps.name("tr").map(|c| c.as_str()),
            pinyin: caps.name("py").unwrap().as_str(),
            oper: caps.name("op").unwrap().as_str(),
            select: caps.name("se").unwrap().as_str(),
        })
    }
}

/// cached input state
#[derive(Debug)]
pub struct InputState {
    pub raw_text: String,
    pub session_id: usize,
    pub offset: usize,
}

pub struct InputResult {
    pub is_new: bool,
    pub select: Option<usize>,
}

impl InputState {
    /// check if cached input is prefix or suffix of current input
    /// return diff result: Add / Delete / New
    pub fn new(raw_text: String, session_id: usize, offset: usize) -> InputState {
        InputState {
            raw_text,
            session_id,
            offset,
        }
    }
    pub fn handle_new_input(
        &self,
        last_input: Input,
        new_offset: usize,
        new_input: &Input,
        rime: &Rime,
    ) -> Result<InputResult, Box<dyn std::error::Error>> {
        // new typing
        if self.offset != new_offset {
            rime.destroy_session(self.session_id);
            return Ok(InputResult {
                is_new: true,
                select: None,
            });
        }
        // continue last typing
        // handle pinyin
        let diff_pinyin = diff(last_input.pinyin, new_input.pinyin);
        match diff_pinyin {
            DiffResult::Add(suffix) => {
                for key in suffix.bytes() {
                    rime.process_key(self.session_id, key as i32);
                }
            }
            DiffResult::Delete(suffix) => {
                for _ in 0..suffix.len() {
                    rime.process_key(self.session_id, 0xff08); // backspace
                }
            }
            DiffResult::New => {
                rime.destroy_session(self.session_id);
            }
            _ => (),
        }
        // TODO: handle PageUP/PageDown operation

        // handle selection
        let idx = match diff_pinyin {
            DiffResult::Delete(_) => None,
            _ if last_input.select.is_empty() && !new_input.select.is_empty() => {
                Some(new_input.select.parse::<usize>().unwrap())
            }
            _ => None,
        };

        Ok(InputResult {
            is_new: matches!(diff_pinyin, DiffResult::New),
            select: idx,
        })
    }
}
