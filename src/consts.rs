/// consts
use lazy_static::lazy_static;
use regex::Regex;

pub const NT_PTN: &str = r"((?P<py>[a-zA-Z[:punct:]]+)(?P<se>[0-9]?))$";
pub const AUTO_TRIGGER_PTN: &str = r"[^a-zA-Z[:punct:]\s][a-zA-Z[:punct:]]+[0-9]?$";
// hack "format argument must be a string literal"
macro_rules! trigger_ptn {
    () => {
        r"((?P<tr>[{}])(?P<py>[a-zA-Z[:punct:]]+)(?P<se>[0-9]?))$"
    };
}
pub(crate) use trigger_ptn;

lazy_static! {
    pub static ref NT_RE: Regex = Regex::new(NT_PTN).unwrap(); // no trigger
    pub static ref AUTO_TRIGGER_RE: Regex = Regex::new(AUTO_TRIGGER_PTN).unwrap(); // no trigger
}

pub const K_BACKSPACE: i32 = 0xff08;
pub const K_PGUP: i32 = 0xff55;
pub const K_PGDN: i32 = 0xff56;
