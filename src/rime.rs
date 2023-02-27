use crate::consts::{KEY_BACKSPACE, RAW_RE};
use librime_sys as librime;
use once_cell::sync::OnceCell;
use std::ffi::{CStr, CString, NulError};
use std::sync::Mutex;
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
static RIME: OnceCell<Rime> = OnceCell::new();

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
    #[error("rime is already initialized")]
    AlreadyInitialized,
    #[error("failed to get candidates")]
    GetCandidatesFailed,
    #[error("session {0} not found")]
    SessionNotFound(usize),
}

#[derive(Debug)]
pub struct RimeResponse {
    /// partially submitted input
    pub submitted: String,
    /// list of candidate provided by rime
    pub candidates: Vec<Candidate>,
}

impl Drop for Rime {
    fn drop(&mut self) {
        // FIXME: it seems that static once_cell variables will not be dropped?
        // self.destroy();
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
    ) -> Result<(), RimeError> {
        if Rime::is_initialized() {
            Err(RimeError::AlreadyInitialized)?
        }
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
        RIME.set(Rime).unwrap();
        Ok(())
    }

    pub fn destroy(&self) {
        if RIME.get().is_some() {
            unsafe {
                librime::RimeCleanupAllSessions();
                librime::RimeFinalize();
            }
        }
    }

    pub fn get_candidates_from_context(
        &self,
        context: &librime::RimeContext,
    ) -> Result<Vec<Candidate>, RimeError> {
        let res = Mutex::new(Vec::new());
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
            let order = (i + 1) as usize;
            res.lock().unwrap().push(Candidate {
                text,
                comment,
                order,
            });
        }
        res.into_inner().map_err(|_| RimeError::GetCandidatesFailed)
    }

    pub fn get_raw_input(&self, session_id: usize) -> Option<String> {
        let api = unsafe { librime::rime_get_api() };
        (!api.is_null())
            .then(|| unsafe { *api })
            .and_then(|api| api.get_input)
            .and_then(|get_input| unsafe {
                let ptr = get_input(session_id);
                CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_owned())
            })
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

    pub fn get_response_from_session(&self, session_id: usize) -> Result<RimeResponse, RimeError> {
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
        let submitted = preedit.map(|s| RAW_RE.replace_all(s.as_ref(), "").to_string());

        // if has commit text, return as the only candidate
        let candidates = if let Some(text) = self.get_commit_text(session_id) {
            Ok(vec![Candidate {
                text,
                comment: "".to_string(),
                order: 0,
            }])
        } else {
            // else get candidates
            self.get_candidates_from_context(&context)
        };
        // free context
        unsafe {
            librime::RimeFreeContext(&mut context);
        }
        candidates.map(|candidates| RimeResponse {
            submitted: submitted.unwrap_or_default(),
            candidates,
        })
    }

    pub fn create_session(&self) -> usize {
        unsafe { librime::RimeCreateSession() }
    }

    /// if session_id does not exist, create a new one
    pub fn find_session(&self, session_id: usize) -> usize {
        unsafe {
            match librime::RimeFindSession(session_id) {
                0 => librime::RimeCreateSession(),
                _ => session_id,
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

    pub fn process_str(&self, session_id: usize, keys: &str) {
        for key in keys.bytes() {
            self.process_key(session_id, key as i32);
        }
    }

    pub fn delete_keys(&self, session_id: usize, len: usize) {
        for _ in 0..len {
            self.process_key(session_id, KEY_BACKSPACE);
        }
    }

    pub fn destroy_session(&self, session_id: usize) {
        unsafe {
            if librime::RimeFindSession(session_id) != 0 {
                librime::RimeDestroySession(session_id);
            }
        }
    }

    pub fn clear_composition(&self, session_id: usize) {
        unsafe {
            librime::RimeClearComposition(session_id);
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
    Rime::init(shared_data_dir, user_data_dir, log_dir).unwrap();
    let rime = Rime::global();
    // simulate typing
    let keys = vec![b'w', b'l', b'h'];
    let session_id = rime.create_session();
    for key in keys {
        rime.process_key(session_id, key as i32);
    }
    let res = rime.get_response_from_session(session_id).unwrap();
    assert!(res.candidates.len() != 0);
    rime.destroy_session(session_id);

    // destroy
    rime.destroy();
}
