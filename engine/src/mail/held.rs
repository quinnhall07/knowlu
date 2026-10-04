//! How long a claim `mail-resolve` never answers is held (Quinn's PQ14, 2026-10-04; plan T6.4).
//!
//! A claim the service leaves unanswered, or whose call fails, holds its message unacked so it is asked
//! again next slot. Held for good it would sit at the head of the server's queue and block every page
//! behind it, so at the [`GIVE_UP_SLOTS`]th slot that asked and got no answer the pass stops asking: the
//! claim is stored as answered "none" (D12), which is the unmatched notice keyed by the claim's own uid,
//! and its message is acked. Never silent.
//!
//! A *slot* is one pass that asked, or tried to ask, about the claim: counted once however many rounds
//! the pass makes. The count lives in `state/mail-resolve-held.json`, generated state a restart reads
//! back: one JSON object, claim uid (`mail:<h>:<n>`) to slots, written through `ledger::dumps_value`
//! beside and renamed over, never a note and never through `write`. An entry goes when its claim is
//! answered, given up on, or no longer asked. A missing or unreadable file is empty: the worst a lost
//! file costs is three more slots of holding.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The slot at which an unanswered claim is given up on: it is held on the slots before it.
pub(super) const GIVE_UP_SLOTS: u32 = 3;

const FILE: &str = "state/mail-resolve-held.json";

/// The slots each still-unanswered claim has been held, by claim uid.
pub(super) struct Held {
    path: PathBuf,
    slots: BTreeMap<String, u32>,
    changed: bool,
}

impl Held {
    pub fn load(vault: &Path) -> Held {
        let path = vault.join(FILE);
        let slots = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<BTreeMap<String, u32>>(&text).ok())
            .unwrap_or_default();
        Held { path, slots, changed: false }
    }

    /// One more slot that asked about `uid` and got no answer: the slots held so far, this one included.
    pub fn strike(&mut self, uid: &str) -> u32 {
        let slots = self.slots.entry(uid.to_string()).or_insert(0);
        *slots += 1;
        self.changed = true;
        *slots
    }

    /// `uid` is answered, given up on or no longer asked: its count goes.
    pub fn clear(&mut self, uid: &str) {
        self.changed |= self.slots.remove(uid).is_some();
    }

    /// Persist what changed (best effort: the next slot counts from whatever is on disk). An empty count
    /// removes the file.
    pub fn save(&mut self) {
        if !std::mem::take(&mut self.changed) {
            return;
        }
        if self.slots.is_empty() {
            let _ = std::fs::remove_file(&self.path);
            return;
        }
        let value = Value::Object(self.slots.iter().map(|(uid, n)| (uid.clone(), Value::from(*n))).collect());
        let tmp = self.path.with_file_name(format!("mail-resolve-held.json.tmp{}", std::process::id()));
        let written = std::fs::create_dir_all(self.path.parent().unwrap_or(Path::new(".")))
            .and_then(|()| std::fs::write(&tmp, format!("{}\n", crate::ledger::dumps_value(&value))))
            .and_then(|()| std::fs::rename(&tmp, &self.path));
        if written.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}
