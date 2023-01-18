/// consts

pub const PTN: &str = r"((?P<py>[a-zA-Z[:punct:]]+)(?P<op>[-=]*)(?P<se>[0-9]?))$";
// hack "format argument must be a string literal"
macro_rules! trg_ptn {
    () => {
        r"((?P<tr>[{}])(?P<py>[a-zA-Z[:punct:]]+)(?P<op>[-=]*)(?P<se>[0-9]?))$"
    };
}
pub(crate) use trg_ptn;

pub const K_BACKSPACE: i32 = 0xff08;
pub const K_PGUP: i32 = 0xff55;
pub const K_PGDN: i32 = 0xff56;
