use crate::consts::{K_PGDN, K_PGUP};
use librime_sys as librime;
use once_cell::sync::OnceCell;
use std::ffi::{CStr, CString, NulError};
use std::sync::RwLock;
use thiserror::Error;

macro_rules! rime_struct_init {
    ($type:ty) => {{
        let mut var: $type = unsafe { std::mem::zeroed() };
        var.data_size =
            (std::mem::size_of::<$type>() - std::mem::size_of_val(&var.data_size)) as i32;
        var
    }};
}

/// global rime instance
pub static RIME: OnceCell<Rime> = OnceCell::new();

/// just call unsafe c ffi function simply
/// TODO: make a good rust wrapper
#[derive(Debug)]
pub struct Rime;

#[derive(Debug)]
pub struct Candidate {
    pub text: String,
    pub comment: String,
    pub order: usize,
}

/// Rime Error Type
#[derive(Error, Debug)]
pub enum RimeError {
    #[error("null pointer when talking with librime")]
    NullPointer(#[from] NulError),
    #[error("failed to get candidates")]
    GetCandidatesFailed,
    #[error("session {0} not found")]
    SessionNotFound(usize),
}

#[derive(Debug)]
pub struct RimeResponse {
    /// length of input accepted by rime
    pub preedit: Option<String>,
    /// list of candidate provided by rime
    pub candidates: Vec<Candidate>,
}

impl Drop for Rime {
    fn drop(&mut self) {
        // FIXME: it seems that static once_cell variables will not be dropped?
        self.destroy();
    }
}

impl Rime {
    /// get global rime instance
    pub fn global() -> &'static Rime {
        RIME.get().expect("Rime is not initialized")
    }

    pub fn is_initialized() -> bool {
        RIME.get().is_some()
    }

    pub fn init(
        shared_data_dir: &str,
        user_data_dir: &str,
        log_dir: &str,
    ) -> Result<Self, RimeError> {
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
                librime::RimeJoinMaintenanceThread();
            }
            // retake pointer
            let _ = CString::from_raw(traits.shared_data_dir as *mut i8);
            let _ = CString::from_raw(traits.user_data_dir as *mut i8);
            #[cfg(not(feature = "no_log_dir"))]
            let _ = CString::from_raw(traits.log_dir as *mut i8);
            let _ = CString::from_raw(traits.distribution_name as *mut i8);
            let _ = CString::from_raw(traits.distribution_code_name as *mut i8);
            let _ = CString::from_raw(traits.distribution_version as *mut i8);
            let _ = CString::from_raw(traits.app_name as *mut i8);
        }
        Ok(Rime)
    }

    pub fn destroy(&self) {
        if RIME.get().is_some() {
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
    ) -> Result<Vec<Candidate>, RimeError> {
        let res = RwLock::new(Vec::new());
        let mut count_pgdn = 0;
        while context.menu.num_candidates != 0 {
            for i in 0..context.menu.num_candidates {
                let candidate = unsafe { *context.menu.candidates.offset(i as isize) };
                let text = unsafe {
                    CStr::from_ptr(candidate.text)
                        .to_str()
                        .map_err(|_| RimeError::GetCandidatesFailed)?
                        .to_owned()
                };
                let comment = unsafe {
                    (!candidate.comment.is_null())
                        .then(|| match CStr::from_ptr(candidate.comment).to_str() {
                            Ok(s) => s.to_string(),
                            Err(_) => "".to_string(),
                        })
                        .unwrap_or_default()
                };
                let order = res.read().unwrap().len() + 1;
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
        res.into_inner().map_err(|_| RimeError::GetCandidatesFailed)
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

    fn get_joined_preedit(&self, context: &librime::RimeContext) -> Option<String> {
        if !context.composition.preedit.is_null() {
            unsafe {
                let preedit = CStr::from_ptr(context.composition.preedit).to_str().ok()?;
                Some(preedit.chars().filter(|c| c != &' ').collect())
            }
        } else {
            None
        }
    }

    pub fn get_response_from_session(
        &self,
        session_id: usize,
        max_candidates: usize,
    ) -> Result<RimeResponse, RimeError> {
        unsafe {
            if librime::RimeFindSession(session_id) == 0 {
                return Err(RimeError::SessionNotFound(session_id));
            }
        }
        // create context
        let mut context = rime_struct_init!(librime::RimeContext);
        unsafe {
            librime::RimeGetContext(session_id, &mut context);
        }
        let preedit = self.get_joined_preedit(&context);
        // if has commit text, return as the only candidate
        let candidates = if let Some(text) = self.get_commit_text(session_id) {
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
        candidates.map(|candidates| RimeResponse {
            preedit,
            candidates,
        })
    }

    #[allow(dead_code)]
    pub fn get_candidates_from_keys(
        &self,
        keys: Vec<u8>,
        max_candidates: usize,
    ) -> Result<Vec<Candidate>, RimeError> {
        let session_id = self.new_session_with_keys(&keys)?;
        let res = self.get_response_from_session(session_id, max_candidates)?;
        self.destroy_session(session_id);
        Ok(res.candidates)
    }

    pub fn new_session_with_keys(&self, keys: &[u8]) -> Result<usize, NulError> {
        let session_id = unsafe { librime::RimeCreateSession() };
        unsafe {
            let ck = CString::new(keys)?.into_raw();
            librime::RimeSimulateKeySequence(session_id, ck);
            let _ = CString::from_raw(ck);
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

    pub fn sync_user_data(&self) {
        unsafe {
            librime::RimeSyncUserData();
            librime::RimeJoinMaintenanceThread();
        }
    }
}

#[test]
fn test_get_candidates() {
    let shared_data_dir = "/usr/share/rime-data/";
    let base_dir = directories::BaseDirs::new().unwrap();
    let data_dir = base_dir.data_dir().join("rime-ls-test");
    let user_data_dir = data_dir.to_str().unwrap();
    let log_dir = "/tmp";

    // init
    let rime = Rime::init(shared_data_dir, user_data_dir, log_dir).unwrap();
    // simulate typing
    let max_candidates = 10;
    let keys = vec![b'w', b'l', b'h'];
    let cands = rime.get_candidates_from_keys(keys, max_candidates).unwrap();
    assert_eq!(cands.len(), max_candidates);
    // destroy
    rime.destroy();
}
