use std::fmt::Display;

use ouroboros::self_referencing;
use regex::Regex;

use crate::consts::KEY_F4;
use crate::rime::Rime;
use crate::utils::{diff, DiffResult};

/// struct that stores matched raw text and its matches
#[self_referencing]
pub struct Input {
    pub raw_text: String,
    #[borrows(raw_text)]
    pub pinyin: &'this str,
    #[borrows(raw_text)]
    pub select: &'this str,
}

impl Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "pinyin: {}, select: {}",
            self.borrow_pinyin(),
            self.borrow_select()
        ))
    }
}

impl Input {
    /// if matches, take ownership of &str, and self-reference it.
    pub fn from_str(re: &Regex, text: &str) -> Option<Self> {
        re.captures(text).map(|caps| {
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

    pub fn is_selecting(&self) -> bool {
        !self.borrow_select().is_empty()
    }
}

/// save input state
pub struct InputState {
    input: Input,
    session_id: usize,
    offset: usize,
    is_incomplete: bool,
}

/// result of handling new input
pub struct InputResult {
    /// session id after handling new input
    pub session_id: usize,
    /// raw input from rime after handling new input
    pub raw_input: Option<String>,
}

impl InputState {
    pub fn new(input: Input, session_id: usize, offset: usize, is_incomplete: bool) -> InputState {
        InputState {
            input,
            session_id,
            offset,
            is_incomplete,
        }
    }

    fn handle_new_typing(session_id: usize, new_input: &Input) -> InputResult {
        let rime = Rime::global();
        rime.process_str(session_id, new_input.borrow_pinyin());
        rime.process_str(session_id, new_input.borrow_select());
        let raw_input = rime.get_raw_input(session_id);

        InputResult {
            session_id,
            raw_input,
        }
    }

    fn handle_schema(session_id: usize) -> InputResult {
        let rime = Rime::global();
        // TODO: support other shortcuts?
        rime.process_key(session_id, KEY_F4);
        let raw_input = rime.get_raw_input(session_id);
        InputResult {
            session_id,
            raw_input,
        }
    }

    pub fn handle_first_input(new_input: &Input, schema_trigger: &str) -> InputResult {
        let rime = Rime::global();
        let session_id = rime.create_session();
        if !schema_trigger.is_empty() && new_input.borrow_pinyin() == &schema_trigger {
            Self::handle_schema(session_id)
        } else {
            Self::handle_new_typing(session_id, new_input)
        }
    }

    pub fn handle_new_input(
        &self,
        new_offset: usize,
        new_input: &Input,
        schema_trigger: &str,
        max_tokens: usize,
    ) -> InputResult {
        let rime = Rime::global();
        let has_session = rime.find_session(self.session_id);
        // new typing (create new session)
        if self.offset != new_offset || !has_session || !self.is_incomplete {
            return Self::handle_first_input(new_input, schema_trigger);
        }
        // continue last typing (with last session)
        let session_id = self.session_id;
        // handle pinyin
        let diff_pinyin = diff(self.input.borrow_pinyin(), new_input.borrow_pinyin());
        match diff_pinyin {
            DiffResult::Add(suffix) => rime.process_str(session_id, suffix),
            DiffResult::Delete(suffix) => {
                // if current pinyin len == max_tokens, force new typing
                if max_tokens > 0 && max_tokens == new_input.borrow_pinyin().len() {
                    rime.clear_composition(session_id);
                    return Self::handle_new_typing(session_id, new_input);
                }
                rime.delete_keys(session_id, suffix.len())
            }
            DiffResult::New => {
                rime.clear_composition(session_id);
                if !schema_trigger.is_empty() && new_input.borrow_pinyin() == &schema_trigger {
                    rime.process_key(session_id, KEY_F4);
                } else {
                    rime.process_str(session_id, new_input.borrow_pinyin());
                }
            }
            _ => (),
        }
        let raw_input = rime.get_raw_input(session_id);
        // handle select
        let diff_select = diff(self.input.borrow_select(), new_input.borrow_select());
        match diff_select {
            DiffResult::Add(suffix) => rime.process_str(session_id, suffix),
            DiffResult::Delete(suffix) => rime.delete_keys(session_id, suffix.len()),
            DiffResult::New => {
                rime.delete_keys(session_id, self.input.borrow_select().len());
                rime.process_str(session_id, new_input.borrow_select());
            }
            _ => (),
        }
        InputResult {
            session_id,
            raw_input,
        }
    }
}
