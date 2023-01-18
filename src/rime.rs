use crate::consts::{K_PGDN, K_PGUP};
use librime_sys as librime;
use std::error::Error;
use std::ffi::{CStr, CString, NulError};
use std::sync::RwLock;

macro_rules! rime_struct_init {
    ($type:ty) => {{
        let mut var: $type = unsafe { std::mem::zeroed() };
        var.data_size =
            (std::mem::size_of::<$type>() - std::mem::size_of_val(&var.data_size)) as i32;
        var
    }};
}

/// just call unsafe c ffi function simply
/// TODO: make a good rust wrapper
#[derive(Debug)]
pub struct Rime {
    is_init: RwLock<bool>,
}

#[derive(Debug)]
pub struct Candidate {
    pub text: String,
    pub comment: String,
    pub order: usize,
}

impl Rime {
    pub fn new() -> Self {
        Rime {
            is_init: RwLock::new(false),
        }
    }

    #[allow(dead_code)]
    pub fn version() -> Option<&'static str> {
        unsafe {
            let api = librime::rime_get_api();
            (*api)
                .get_version
                .and_then(|f| CStr::from_ptr(f()).to_str().ok())
        }
    }

    pub fn init(
        &self,
        shared_data_dir: &str,
        user_data_dir: &str,
        log_dir: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut traits = rime_struct_init!(librime::RimeTraits);

        // set dirs
        traits.shared_data_dir = CString::new(shared_data_dir)?.into_raw();
        traits.user_data_dir = CString::new(user_data_dir)?.into_raw();

        #[cfg(not(feature = "no_log_dir"))]
        {
            traits.log_dir = CString::new(log_dir)?.into_raw();
            traits.min_log_level = 2; // ERROR
        }

        // set name
        traits.distribution_name = CString::new("Rime")?.into_raw();
        traits.distribution_code_name = CString::new("rime-ls")?.into_raw();
        traits.distribution_version = CString::new(env!("CARGO_PKG_VERSION"))?.into_raw();
        traits.app_name = CString::new("rime.rime-ls")?.into_raw();

        unsafe {
            librime::RimeSetup(&mut traits);
            librime::RimeInitialize(&mut traits);
            if librime::RimeStartMaintenanceOnWorkspaceChange() != 0 {
                librime::RimeJoinMaintenanceThread();
                librime::RimeSyncUserData();
            }
        }

        *self.is_init.write().unwrap() = true;
        Ok(())
    }

    pub fn destroy(&self) {
        if *self.is_init.read().unwrap() {
            unsafe {
                librime::RimeFinalize();
            }
        }
    }

    pub fn get_candidates_from_context(
        &self,
        session_id: usize,
        context: &mut librime::RimeContext,
        max_candidates: usize,
    ) -> Result<Vec<Candidate>, Box<dyn Error>> {
        let res = RwLock::new(Vec::new());
        let mut count_pgdn = 0;
        while context.menu.num_candidates != 0 {
            for i in 0..context.menu.num_candidates {
                let candidate = unsafe { *context.menu.candidates.offset(i as isize) };
                let text = unsafe { CStr::from_ptr(candidate.text).to_str()?.to_owned() };
                let comment = unsafe {
                    (!candidate.comment.is_null())
                        .then(|| match CStr::from_ptr(candidate.comment).to_str() {
                            Ok(s) => s.to_string(),
                            Err(_) => "".to_string(),
                        })
                        .unwrap_or_default()
                };
                let order = res.read().unwrap().len();
                res.write().unwrap().push(Candidate {
                    text,
                    comment,
                    order,
                });
                if res.read().unwrap().len() >= max_candidates {
                    break;
                }
            }

            if res.read().unwrap().len() >= max_candidates {
                break;
            }
            if context.menu.is_last_page != 0 {
                break;
            }
            // pagedown
            unsafe {
                if librime::RimeProcessKey(session_id, K_PGDN, 0) == 0 {
                    break;
                }
                count_pgdn += 1;
                librime::RimeGetContext(session_id, context);
            }
        }
        // page_up to resume session
        for _ in 0..count_pgdn {
            unsafe {
                librime::RimeProcessKey(session_id, K_PGUP, 0);
            }
        }
        Ok(res.into_inner()?)
    }

    fn get_commit_text(&self, session_id: usize) -> Option<String> {
        let mut commit = rime_struct_init!(librime::RimeCommit);
        let mut ans: Option<String> = None;
        unsafe {
            librime::RimeGetCommit(session_id, &mut commit);
            if !commit.text.is_null() {
                ans = CStr::from_ptr(commit.text)
                    .to_str()
                    .ok()
                    .map(|s| s.to_string());
            }
            librime::RimeFreeCommit(&mut commit);
        }
        ans
    }

    pub fn get_candidates_from_session(
        &self,
        session_id: usize,
        max_candidates: usize,
    ) -> Result<Vec<Candidate>, Box<dyn Error>> {
        unsafe {
            if librime::RimeFindSession(session_id) == 0 {
                Err("No such session")?
            }
        }
        // create context
        let mut context = rime_struct_init!(librime::RimeContext);
        unsafe {
            librime::RimeGetContext(session_id, &mut context);
        }
        // if has commit text, return as the only candidate
        let res = if let Some(text) = self.get_commit_text(session_id) {
            Ok(vec![Candidate {
                text,
                comment: "".to_string(),
                order: 0,
            }])
        } else {
            // else get candidates
            // TODO: based on total num_candidates, does not obey rime's pages
            // FIXME: ugly code
            self.get_candidates_from_context(session_id, &mut context, max_candidates)
        };
        // free context
        unsafe {
            librime::RimeFreeContext(&mut context);
        }
        res
    }

    #[allow(dead_code)]
    pub fn get_candidates_from_keys(
        &self,
        keys: Vec<u8>,
        max_candidates: usize,
    ) -> Result<Vec<Candidate>, Box<dyn Error>> {
        let session_id = self.new_session_with_keys(&keys)?;
        let res = self.get_candidates_from_session(session_id, max_candidates)?;
        self.destroy_session(session_id);
        Ok(res)
    }

    pub fn new_session_with_keys(&self, keys: &[u8]) -> Result<usize, NulError> {
        let session_id = unsafe { librime::RimeCreateSession() };
        unsafe {
            let ck = CString::new(keys)?;
            librime::RimeSimulateKeySequence(session_id, ck.into_raw());
        }
        Ok(session_id)
    }

    pub fn destroy_session(&self, session_id: usize) {
        unsafe {
            if librime::RimeFindSession(session_id) != 0 {
                librime::RimeDestroySession(session_id);
            }
        }
    }

    pub fn process_key(&self, session_id: usize, key: i32) {
        unsafe {
            if librime::RimeFindSession(session_id) != 0 {
                librime::RimeProcessKey(session_id, key, 0);
            }
        }
    }
}

#[test]
fn test_get_candidates() {
    let shared_data_dir = "/usr/share/rime-data/";
    let user_data_dir = "/home/wlh/.local/share/rime-ls/";
    let log_dir = "/tmp";
    // init
    let rime = Rime::new();
    rime.init(shared_data_dir, user_data_dir, log_dir).unwrap();
    // simulate typing
    let max_candidates = 10;
    let keys = vec![b'w', b'l', b'h'];
    let cands = rime.get_candidates_from_keys(keys, max_candidates).unwrap();
    assert_eq!(cands.len(), max_candidates);
    // destroy
    rime.destroy();
}
