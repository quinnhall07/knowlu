//! Heads-up items for forwarded mailboxes (spec §4.10's *Shown*, §4.11 step 2; plan T6.3).
//!
//! The pull's answer names each mailbox by `address_id` and its state, as data. Per non-active mailbox
//! the pass keeps **one** `heads-up` item (`close_key: mail-source:<address_id>`), closed when the mailbox
//! is active again, and names the state in one run line. When the saved session turned forwarding back on
//! (`reenabled_at`), one `mail-reenabled:<address_id>` item says so, once per stamp: the stamp is
//! recorded in `state/ingest-seen.md` once the item is written, so a student's dismissal sticks and a failed
//! write is tried again. No address, no token and no mail
//! text is written or printed; the words below are fixed.

use std::path::Path;

use crate::cloudmodel::{MailboxReport, MailboxState};
use crate::info;
use crate::ingest;
use crate::yaml;

use super::Writer;

/// §4.11 step 2's text, exactly.
pub(super) const REENABLED_TEXT: &str = "Email forwarding was off; Knowlu turned it back on. Turn off email in Settings to stop this.";

/// The title and the sentence a non-active state shows, and the word the run line names it by. `None`
/// for a state that is no trouble: active, and the two a setup passes through.
fn trouble(state: MailboxState) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match state {
        MailboxState::Quiet => ("quiet", "Email forwarding has gone quiet", "No forwarded mail has arrived for a while. Knowlu is checking that forwarding still works."),
        MailboxState::Blocked => ("blocked", "Email forwarding is blocked", "Forwarding is set up, but the delivery check never arrived. Open Settings to finish connecting email."),
        MailboxState::Off => ("off", "Email forwarding is off", "Forwarding in your mail is off or goes somewhere else. Open Settings to turn it back on."),
        MailboxState::PausedSend => ("paused", "Email setup is paused", "Your school mail is limiting how fast Knowlu can send. Setup carries on by itself."),
        MailboxState::Lapsed => ("lapsed", "Email forwarding has lapsed", "Knowlu lost its connection to this mailbox. Open Settings to reconnect it."),
        MailboxState::Revoked => ("revoked", "Email access was revoked", "Open Settings to connect this mailbox again."),
        MailboxState::Unknown => ("unknown", "Email forwarding needs a look", "Open Settings to see this mailbox's state."),
        MailboxState::Active | MailboxState::Pending | MailboxState::Confirmed => return None,
    })
}

fn is_open(vault: &Path, key: &str) -> bool {
    info::list_info(vault).iter().any(|item| yaml::opt_text(yaml::get(item, "close_key")).as_deref() == Some(key))
}

/// Keep the heads-up items for `boxes` (in `address_id` order) and return the named run lines. A write
/// that fails is one line and never stops the pass.
pub(super) fn keep(writer: &mut Writer<'_>, vault: &Path, boxes: &[&MailboxReport], stamp: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for mailbox in boxes {
        let id = &mailbox.address_id;
        let key = format!("mail-source:{id}");
        match trouble(mailbox.state) {
            Some((word, title, body)) => {
                lines.push(format!("mail: mailbox {id} is {word}"));
                if !is_open(vault, &key) {
                    note(writer, &mut lines, &key, title, body);
                }
            }
            None if mailbox.state == MailboxState::Active => {
                if is_open(vault, &key) && writer.close_notices(&key).is_err() {
                    lines.push("mail: a heads-up could not be closed".to_string());
                }
            }
            None => {}
        }
        if let Some(at) = mailbox.reenabled_at {
            let (key, stamped) = (format!("mail-reenabled:{id}"), format!("mail-reenabled:{id}:{at}"));
            if !is_open(vault, &key) && !ingest::load_seen(vault).contains(&stamped) {
                // Stamped only once the item is written: a failed write is tried again next pass.
                let written = note(writer, &mut lines, &key, "Email forwarding was turned back on", REENABLED_TEXT);
                if written && ingest::record_seen(vault, &stamped, "(email)", stamp).is_err() {
                    lines.push("mail: the seen ledger was not written".to_string());
                }
            }
        }
    }
    lines
}

/// Write one heads-up; `false` (and one line) when the write failed.
fn note(writer: &mut Writer<'_>, lines: &mut Vec<String>, key: &str, title: &str, body: &str) -> bool {
    let written = writer.notice("heads-up", Some(key), title, body).is_ok();
    if !written {
        lines.push("mail: a heads-up could not be written".to_string());
    }
    written
}
