use ouroboros::self_referencing;
use regex::Regex;

use crate::consts::KEY_F4;
use crate::rime::Rime;
use crate::utils::{self, DiffResult};

/// struct that stores matched raw text and its matches
#[self_referencing]
struct InputInternal {
    pub raw_text: String,
    #[borrows(raw_text)]
    pub pinyin: &'this str,
    #[borrows(raw_text)]
    pub select: &'this str,
}

impl InputInternal {
    /// if matches, take ownership of &str, and self-reference it.
    pub fn from_str(re: &Regex, text: &str) -> Option<Self> {
        re.captures(text).map(|caps| {
            let start = caps.get(0).unwrap().start();
            let raw_text = caps.get(0).unwrap().as_str().to_owned();
            InputInternalBuilder {
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

pub struct Input {
    internal: InputInternal,
    is_schema: bool,
}

impl Input {
    pub fn new(re: &Regex, text: &str, schema_trigger: &str) -> Option<Self> {
        InputInternal::from_str(re, text).map(|internal| {
            let is_schema = utils::is_schema_triggered(internal.borrow_pinyin(), schema_trigger);
            Input {
                internal,
                is_schema,
            }
        })
    }

    pub fn raw_text(&self) -> &str {
        self.internal.borrow_raw_text()
    }

    pub fn pinyin(&self) -> &str {
        self.internal.borrow_pinyin()
    }

    pub fn select(&self) -> &str {
        self.internal.borrow_select()
    }

    pub fn is_schema(&self) -> bool {
        self.is_schema
    }
    pub fn is_selecting(&self) -> bool {
        !self.internal.borrow_select().is_empty()
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
    /// sometimes extra offset is caused by new input
    pub extra_offset: usize,
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

    #[inline]
    fn process_pinyin(rime: &Rime, session_id: usize, input: &Input) {
        if input.is_schema() {
            // TODO: support other shortcuts?
            rime.process_key(session_id, KEY_F4);
        } else {
            rime.process_str(session_id, input.pinyin());
        }
    }

    #[inline]
    fn assemble_result(session_id: usize, pinyin: &str, raw_input: Option<String>) -> InputResult {
        let extra_offset = raw_input
            .and_then(utils::option_string)
            .and_then(|rime_raw_input| pinyin.rfind(&rime_raw_input))
            .unwrap_or(0);
        InputResult {
            session_id,
            extra_offset,
        }
    }

    pub fn first_input(new_input: &Input) -> InputResult {
        let rime = Rime::global();
        let session_id = rime.create_session();

        Self::process_pinyin(rime, session_id, new_input);
        rime.process_str(session_id, new_input.select());

        let raw_input = rime.get_raw_input(session_id);
        Self::assemble_result(session_id, new_input.pinyin(), raw_input)
    }

    fn continue_input(&self, new_input: &Input, max_tokens: usize) -> InputResult {
        let rime = Rime::global();
        let session_id = self.session_id;

        let old_pinyin = self.input.pinyin();
        let pinyin = new_input.pinyin();
        let old_select = self.input.select();
        let select = new_input.select();

        match utils::diff(old_pinyin, pinyin) {
            DiffResult::Add(suffix) => rime.process_str(session_id, suffix),
            DiffResult::Delete(suffix) => {
                // if current pinyin len == max_tokens, force new typing
                if max_tokens > 0 && max_tokens == pinyin.len() {
                    rime.clear_composition(session_id);
                    Self::process_pinyin(rime, session_id, new_input);
                    rime.process_str(session_id, select);
                }
                rime.delete_keys(session_id, suffix.len())
            }
            DiffResult::New => {
                rime.clear_composition(session_id);
                Self::process_pinyin(rime, session_id, new_input);
            }
            _ => (),
        }
        // get raw input before select or we may get empty string
        let raw_input = rime.get_raw_input(session_id);

        match utils::diff(old_select, select) {
            DiffResult::Add(suffix) => rime.process_str(session_id, suffix),
            DiffResult::Delete(suffix) => rime.delete_keys(session_id, suffix.len()),
            DiffResult::New => {
                rime.delete_keys(session_id, old_select.len());
                rime.process_str(session_id, select);
            }
            _ => (),
        }
        Self::assemble_result(session_id, pinyin, raw_input)
    }

    pub fn apply_input(&self, new_offset: usize, input: &Input, max_tokens: usize) -> InputResult {
        let rime = Rime::global();
        // 1. totally new typing (create new session)
        if !rime.find_session(self.session_id) {
            return Self::first_input(input);
        }
        // 2. typing with new offset (destroy old session and create new one)
        if self.offset != new_offset || !self.is_incomplete {
            rime.destroy_session(self.session_id);
            return Self::first_input(input);
        }
        // 3. continue last typing, diff and process (with last session)
        self.continue_input(input, max_tokens)
    }
}
