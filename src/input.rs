use crate::rime::Rime;
use crate::utils::{DiffResult, diff};
use lazy_static::lazy_static;
use regex::Regex;

/// regex raw input to groups
#[derive(Debug)]
pub struct Input<'s> {
    pub raw_text: &'s str,
    pub pinyin: &'s str,
    pub oper: &'s str,
    pub select: &'s str,
}

const PTN: &str = r"((?P<py>[a-zA-Z]+)(?P<op>[-=]?)(?P<se>[0-9]?))$";

lazy_static! {
    static ref INPUT_RE: Regex = Regex::new(PTN).unwrap();
}

impl<'s> Input<'s> {
    pub fn from_str(text: &'s str) -> Option<Self> {
        INPUT_RE.captures(text).map(|caps| Input {
            raw_text: caps.get(0).unwrap().as_str(),
            pinyin: caps.name("py").unwrap().as_str(),
            oper: caps.name("op").unwrap().as_str(),
            select: caps.name("se").unwrap().as_str(),
        })
    }
}

/// cached input state
#[derive(Debug, Default)]
pub struct InputState {
    pub session_id: usize,
    pub offset: usize,
    pub raw_text: String,
}


impl InputState {
    /// check if cached input is prefix or suffix of current input
    /// return diff result: Add / Delete / New
    pub fn handle_new_input(
        &mut self,
        new_offset: usize,
        new_input: &Input,
        rime: &Rime,
    ) -> Result<Option<usize>, Box<dyn std::error::Error>> {
        // new typing
        // dbg!(new_input);
        if self.raw_text.is_empty() || self.offset != new_offset {
            rime.destroy_session(self.session_id);
            self.session_id = rime.new_session_with_keys(new_input.pinyin.as_bytes())?;
            self.raw_text = new_input.raw_text.to_string();
            self.offset = new_offset;
            return Ok(None);
        }
        // continue last typing
        let last_input = Input::from_str(&self.raw_text).unwrap();
        // handle pinyin
        match diff(last_input.pinyin, new_input.pinyin) {
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
                self.session_id = rime.new_session_with_keys(new_input.pinyin.as_bytes())?;
            }
        }
        // TODO: handle PageUP/PageDown operation

        // handle selection
        let idx = if last_input.select.is_empty() && !new_input.select.is_empty() {
            // do selection
            Some(new_input.select.parse::<usize>().unwrap())
        } else {
            None
        };

        self.raw_text = new_input.raw_text.to_string();
        self.offset = new_offset;
        Ok(idx)
    }
}
