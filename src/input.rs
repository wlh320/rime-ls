use crate::consts::K_BACKSPACE;
use crate::rime::Rime;
use crate::utils::{diff, DiffResult};
use ouroboros::self_referencing;
use regex::Regex;
use std::borrow::Cow;

/// struct that stores matched raw text and its matches
#[self_referencing]
pub struct Input {
    pub raw_text: String,
    #[borrows(raw_text)]
    pub pinyin: &'this str,
    #[borrows(raw_text)]
    pub select: &'this str,
}

impl Input {
    /// if matches, take ownership of &str, and self-reference it.
    pub fn from_str(re: &Regex, text: Cow<str>) -> Option<Self> {
        re.captures(&text).map(|caps| {
            let start = caps.get(0).unwrap().start();
            let raw_text = caps.get(0).unwrap().as_str().to_owned();
            InputBuilder {
                raw_text,
                pinyin_builder: |raw_text| {
                    let m = caps.name("py").unwrap();
                    &raw_text[m.start() - start..m.end() - start]
                },
                select_builder: |raw_text| {
                    let m = caps.name("se").unwrap();
                    &raw_text[m.start() - start..m.end() - start]
                },
            }
            .build()
        })
    }
}

/// save input state
pub struct InputState {
    pub input: Input,
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
    pub fn new(input: Input, session_id: usize, offset: usize) -> InputState {
        InputState {
            input,
            session_id,
            offset,
        }
    }

    pub fn handle_new_input(
        &self,
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
        let diff_pinyin = diff(self.input.borrow_pinyin(), new_input.borrow_pinyin());
        match diff_pinyin {
            DiffResult::Add(suffix) => {
                for key in suffix.bytes() {
                    rime.process_key(self.session_id, key as i32);
                }
            }
            DiffResult::Delete(suffix) => {
                for _ in 0..suffix.len() {
                    rime.process_key(self.session_id, K_BACKSPACE);
                }
            }
            DiffResult::New => {
                rime.destroy_session(self.session_id);
            }
            _ => (),
        }
        // handle selection
        let idx = match diff_pinyin {
            DiffResult::Delete(_) => None,
            _ if self.input.borrow_select().is_empty() && !new_input.borrow_select().is_empty() => {
                Some(new_input.borrow_select().parse::<usize>().unwrap())
            }
            _ => None,
        };

        Ok(InputResult {
            is_new: matches!(diff_pinyin, DiffResult::New),
            select: idx,
        })
    }
}
