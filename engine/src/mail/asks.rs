//! Ambiguity asked of the service (spec §4.8 step 3, D12; plan T6.4).
//!
//! A claim the plan leaves ambiguous ([`mailreconcile::resolve::to_ask`]) goes to `mail-resolve` in one
//! batch per course, before [`mailreconcile::plan`] runs. The call carries the claims as references
//! (message uid and index: the service holds their text) and the course's items as (id, title, due), the
//! only vault text it sends. Each answer is set on its claim as the stored [`Resolution`] the service
//! also keeps, so the plan reads it as it reads any stored answer: a chosen item is a change to it, and
//! "none" is a notice, never a guess. A claim the service did not answer, or any claim after a failed
//! call, stays ambiguous: it writes nothing and its message is held, so it is asked again next slot.

use std::collections::BTreeMap;

use crate::cloudmodel::{self, Candidate, CloudClient, ResolveBatch};
use crate::mailreconcile::{ClaimRef, Message, Resolution, VaultView};

/// Ask once per course about `asked`, and store each answer on its claim in `messages`. `may_call` is the
/// budget's say before each request. Returns the closed-set word for what ended the asking early (a
/// failed call or no time left), which names no message text.
pub(super) fn settle(
    client: &CloudClient,
    view: &VaultView,
    messages: &mut [Message],
    asked: &[ClaimRef],
    mut may_call: impl FnMut() -> bool,
) -> Option<&'static str> {
    let mut by_course: BTreeMap<String, Vec<ClaimRef>> = BTreeMap::new();
    for at in asked {
        let course = messages.iter().find(|m| m.uid == at.message).and_then(|m| m.claims.get(at.index)).and_then(|c| c.course.clone());
        if let Some(course) = course {
            by_course.entry(course).or_default().push(at.clone());
        }
    }
    for (course, claims) in by_course {
        if !may_call() {
            return Some("out of time");
        }
        let batch = ResolveBatch { candidates: candidates(view, &course), course, claims };
        let answers = match cloudmodel::resolve_mail(client, &batch) {
            Ok(answers) => answers,
            Err(e) => return Some(e.label()),
        };
        for answer in answers {
            let claim = messages.iter_mut().find(|m| m.uid == answer.claim.message).and_then(|m| m.claims.get_mut(answer.claim.index));
            if let Some(claim) = claim {
                claim.resolution = Some(Resolution { target_id: answer.target_id });
            }
        }
    }
    None
}

/// The course's items, in the view's order: the notes a stored answer may name (`tasks/` and `archive/`).
fn candidates(view: &VaultView, course: &str) -> Vec<Candidate> {
    view.notes
        .iter()
        .filter(|n| matches!(n.folder.as_str(), "tasks" | "archive") && n.course.as_deref() == Some(course))
        .map(|n| Candidate { id: n.id.clone(), title: n.title.clone(), due: n.due.as_str().map(str::to_string) })
        .collect()
}
