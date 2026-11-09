//! P1t's shared fakes: an injected credential store and a file seam for the marks, both recording
//! every call in order into one log. Included by `#[path]` from `school_password.rs` and
//! `school_fill.rs`. Invented values only; nothing here touches Credential Manager or the disk.
#![allow(dead_code)]
use knowlu::school_password::{DeleteError, SchoolMarks, SchoolStore};
use knowlu::school_session::try_take;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub type Log = Rc<RefCell<Vec<String>>>;

pub fn new_log() -> Log {
    Rc::new(RefCell::new(Vec::new()))
}

/// Index of the first log entry that starts with `prefix`.
pub fn at(log: &Log, prefix: &str) -> Option<usize> {
    log.borrow().iter().position(|e| e.starts_with(prefix))
}

pub fn count(log: &Log, prefix: &str) -> usize {
    log.borrow().iter().filter(|e| e.starts_with(prefix)).count()
}

/// Store and marks in one value, so one log holds the order across both.
pub struct Fake {
    pub log: Log,
    /// The profile whose lock each call probes: a call made while it can be taken is "unlocked".
    pub profile: String,
    pub unlocked: RefCell<Vec<String>>,
    pub entries: RefCell<HashMap<String, (String, String)>>,
    pub rejected: Cell<bool>,
    pub in_flight: Cell<bool>,
    /// Call names that fail: `write`, `delete`, `set_rejected`, `clear_rejected`, `write_in_flight`,
    /// `flush_in_flight`, `clear_in_flight`.
    pub fail: RefCell<HashSet<&'static str>>,
    /// When set, a failing `write` carries the secret in its error text, as a careless store could.
    pub echo_secret: Cell<bool>,
}

impl Fake {
    pub fn new(profile: &str) -> Fake {
        Fake {
            log: new_log(),
            profile: profile.to_string(),
            unlocked: RefCell::new(Vec::new()),
            entries: RefCell::new(HashMap::new()),
            rejected: Cell::new(false),
            in_flight: Cell::new(false),
            fail: RefCell::new(HashSet::new()),
            echo_secret: Cell::new(false),
        }
    }
    pub fn failing(self, names: &[&'static str]) -> Fake {
        self.fail.borrow_mut().extend(names.iter().copied());
        self
    }
    pub fn with_entry(self, target: &str, user: &str, secret: &str) -> Fake {
        self.entries.borrow_mut().insert(target.to_string(), (user.to_string(), secret.to_string()));
        self
    }
    fn note(&self, entry: String) {
        if try_take(&self.profile).is_some() {
            self.unlocked.borrow_mut().push(entry.clone());
        }
        self.log.borrow_mut().push(entry);
    }
    fn fails(&self, name: &str) -> bool {
        self.fail.borrow().contains(name)
    }
    fn mark(&self, name: &'static str, apply: impl FnOnce()) -> Result<(), String> {
        self.note(name.to_string());
        if self.fails(name) {
            return Err(format!("{name} failed"));
        }
        apply();
        Ok(())
    }
}

impl SchoolStore for Fake {
    fn write(&self, target: &str, user: &str, secret: &str) -> Result<(), String> {
        self.note(format!("write:{target}"));
        if self.fails("write") {
            return Err(if self.echo_secret.get() {
                format!("credential write failed for {target}: {secret}")
            } else {
                format!("credential write failed for {target}: 5")
            });
        }
        self.entries.borrow_mut().insert(target.to_string(), (user.to_string(), secret.to_string()));
        Ok(())
    }
    fn read(&self, target: &str) -> Result<(String, String), String> {
        self.note(format!("read:{target}"));
        self.entries.borrow().get(target).cloned().ok_or_else(|| "no such credential".to_string())
    }
    fn exists(&self, target: &str) -> bool {
        self.note(format!("exists:{target}"));
        self.entries.borrow().contains_key(target)
    }
    fn delete(&self, target: &str) -> Result<(), DeleteError> {
        self.note(format!("delete:{target}"));
        if self.fails("delete") {
            return Err(DeleteError::Failed(format!("credential delete failed for {target}: 5")));
        }
        // Like `credentials::delete`: a target that is not there is the store's own not-found, an
        // answer of its own and never inferred from `exists` (contract review r1, Minor 2).
        self.entries.borrow_mut().remove(target).map(|_| ()).ok_or(DeleteError::NotFound)
    }
}

impl SchoolMarks for Fake {
    fn rejected(&self) -> bool {
        self.note("rejected?".to_string());
        self.rejected.get()
    }
    fn in_flight(&self) -> bool {
        self.note("in_flight?".to_string());
        self.in_flight.get()
    }
    fn set_rejected(&self) -> Result<(), String> {
        self.mark("set_rejected", || self.rejected.set(true))
    }
    fn clear_rejected(&self) -> Result<(), String> {
        self.mark("clear_rejected", || self.rejected.set(false))
    }
    fn write_in_flight(&self) -> Result<(), String> {
        self.mark("write_in_flight", || self.in_flight.set(true))
    }
    fn flush_in_flight(&self) -> Result<(), String> {
        self.mark("flush_in_flight", || {})
    }
    fn clear_in_flight(&self) -> Result<(), String> {
        self.mark("clear_in_flight", || self.in_flight.set(false))
    }
}
