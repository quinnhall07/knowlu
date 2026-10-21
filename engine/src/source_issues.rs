//! Source issues (L7, plan 2026-10-19 §3.4): a `heads-up` in `info/` that names a source Knowlu
//! cannot read or a service it cannot reach, kept as **one** open item per key.
//!
//! Two callers raise and clear them, both in `enrich`: the judge pass (`judge-service:out-of-credit`)
//! and the Gmail pull (`gmail-source:quiet`). Email forwarding's `mail-source:<address>` items are
//! written by `mail`, not here, but are recognised here so the read model puts them in one place.
//! Everything goes through `info::open_info` and `info::close_info`, so every write is journalled
//! first and a failure is returned, never swallowed. No model, no network, no clock of its own.

use std::path::Path;

use crate::info::{self, InfoError, NewInfo};
use crate::journal::Journal;
use crate::write::WriteContext;

/// The judgment service answered that Knowlu's own provider account is out of credit.
pub const OUT_OF_CREDIT_KEY: &str = "judge-service:out-of-credit";
/// A connected Gmail in which Knowlu has found no mail for a week.
pub const GMAIL_QUIET_KEY: &str = "gmail-source:quiet";

/// `mail-reenabled:` is absent on purpose: it is an announcement, not trouble.
const PREFIXES: [&str; 3] = ["mail-source:", "judge-service:", "gmail-source:"];

/// Whether an `info/` item is a source issue: kind `heads-up` and a `close_key` with a listed prefix.
pub fn is_source_issue(kind: &str, close_key: &str) -> bool {
    kind == "heads-up" && PREFIXES.iter().any(|prefix| close_key.starts_with(prefix))
}

fn key_of(meta: &serde_yaml_ng::Mapping) -> Option<String> {
    crate::yaml::opt_text(crate::yaml::get(meta, "close_key"))
}

/// Open one `heads-up` under `key`, opened by `ctx.actor`, unless an open item already carries the
/// key. Returns whether a note was written. It never duplicates and never re-opens a closed twin.
pub fn raise(
    vault: &Path,
    key: &str,
    title: &str,
    sentence: &str,
    ctx: &WriteContext,
    journal: &mut Journal,
) -> Result<bool, InfoError> {
    if info::list_info(vault).iter().any(|m| key_of(m).as_deref() == Some(key)) {
        return Ok(false);
    }
    let item = NewInfo {
        title, kind: "heads-up", body: sentence, opened_by: &ctx.actor, close_key: Some(key), expires: None,
    };
    info::open_info(vault, &item, ctx, Some(journal), None)?;
    Ok(true)
}

/// Close every open item carrying `key`, as `ctx.actor`. Returns how many. None open writes nothing.
pub fn clear(vault: &Path, key: &str, ctx: &WriteContext, journal: &mut Journal) -> Result<usize, InfoError> {
    if !info::list_info(vault).iter().any(|m| key_of(m).as_deref() == Some(key)) {
        return Ok(0);
    }
    Ok(info::close_info(vault, Some(key), None, &ctx.actor, ctx, Some(journal), None)?.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> WriteContext {
        WriteContext { actor: "agent:knowlu.enrich".into(), via: "local-runner".into(), run_id: None }
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qo-source-issues-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The journal's total size in bytes: unchanged exactly when nothing was written.
    fn journal_bytes(vault: &Path) -> u64 {
        std::fs::read_dir(vault.join("state").join("journal"))
            .map(|d| d.filter_map(|e| e.ok()?.metadata().ok()).map(|m| m.len()).sum())
            .unwrap_or(0)
    }

    #[test]
    fn raising_twice_leaves_one_open_item() {
        let vault = scratch("twice");
        let mut journal = Journal::new(&vault);
        assert!(raise(&vault, OUT_OF_CREDIT_KEY, "Knowlu's AI checks are paused", "On our side.", &ctx(), &mut journal).unwrap());
        assert!(!raise(&vault, OUT_OF_CREDIT_KEY, "Knowlu's AI checks are paused", "On our side.", &ctx(), &mut journal).unwrap());
        let open = info::list_info(&vault);
        assert_eq!(open.len(), 1, "{open:?}");
        assert_eq!(crate::yaml::opt_text(crate::yaml::get(&open[0], "opened_by")).as_deref(), Some("agent:knowlu.enrich"));
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn clearing_closes_every_open_twin_and_none_writes_nothing() {
        let vault = scratch("clear");
        let mut journal = Journal::new(&vault);
        assert_eq!(clear(&vault, GMAIL_QUIET_KEY, &ctx(), &mut journal).unwrap(), 0);
        assert_eq!(journal_bytes(&vault), 0, "nothing open: the journal is untouched");
        raise(&vault, GMAIL_QUIET_KEY, "Gmail has gone quiet", "A week.", &ctx(), &mut journal).unwrap();
        let before = journal_bytes(&vault);
        assert_eq!(clear(&vault, OUT_OF_CREDIT_KEY, &ctx(), &mut journal).unwrap(), 0, "another key is left alone");
        assert_eq!(info::list_info(&vault).len(), 1);
        assert_eq!(journal_bytes(&vault), before);
        assert_eq!(clear(&vault, GMAIL_QUIET_KEY, &ctx(), &mut journal).unwrap(), 1);
        assert!(info::list_info(&vault).is_empty());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn a_source_issue_is_a_heads_up_with_a_listed_prefix() {
        for key in ["mail-source:addr_1", "judge-service:out-of-credit", "gmail-source:quiet"] {
            assert!(is_source_issue("heads-up", key), "{key}");
            assert!(!is_source_issue("notice", key), "{key} as a notice");
        }
        assert!(!is_source_issue("heads-up", "mail-reenabled:addr_1"), "an announcement, not trouble");
        assert!(!is_source_issue("heads-up", "gmail-thread:1a04a6"));
    }
}
