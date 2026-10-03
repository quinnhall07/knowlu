//! Ambiguity asked of the service (spec §4.8 step 3, D12; plan T6.4).
//!
//! A claim the plan leaves ambiguous ([`mailreconcile::resolve::to_ask`]) goes to `mail-resolve` in one
//! batch per course, before [`mailreconcile::plan`] runs. The call carries the claims as references
//! (message uid and index: the service holds their text) and the course's items as (id, title, due), the
//! only vault text it sends. Each answer is set on its claim as the stored [`Resolution`] the service
//! also keeps, so the plan reads it as it reads any stored answer: a chosen item is a change to it, and
//! "none" is a notice, never a guess. A claim the service did not answer stays ambiguous: it writes
//! nothing and its message is held, so it is asked again next slot. A course the service refuses for
//! itself (a 4xx other than 401, 402, 403, 408 and 429) is skipped and the next course is still asked; any
//! other failure (session, entitlement, rate, 5xx, no network, no time) ends the asking for the pass.
//! There is no give-up path: a claim the service never answers stays held (plan ledger, T5.5 row).

use std::collections::BTreeMap;

use crate::cloudmodel::{self, Candidate, CloudClient, CloudError, ResolveBatch};
use crate::mailreconcile::{ClaimRef, Message, Resolution, VaultView};

/// What [`settle`] could not finish, as closed-set words that name no message text.
#[derive(Default)]
pub(super) struct Unsettled {
    /// What ended the asking early (a failed call that every course would meet, or no time left).
    pub ended: Option<&'static str>,
    /// A course the service refused for itself and was skipped: the courses after it were still asked.
    pub refused: Option<&'static str>,
}

/// A refusal of one batch (a 4xx that is not about the session, the entitlement, the rate or the clock):
/// the next course is a different request, so it is still asked.
fn batch_refusal(e: &CloudError) -> bool {
    matches!(e, CloudError::Status { code: 400..=499, .. }) && !e.fatal() && !matches!(e, CloudError::Status { code: 408 | 429, .. })
}

/// Ask once per course about `asked`, and store each answer on its claim in `messages`. `may_call` is the
/// budget's say before each request.
pub(super) fn settle(
    client: &CloudClient,
    view: &VaultView,
    messages: &mut [Message],
    asked: &[ClaimRef],
    mut may_call: impl FnMut() -> bool,
) -> Unsettled {
    let mut unsettled = Unsettled::default();
    let mut by_course: BTreeMap<String, Vec<ClaimRef>> = BTreeMap::new();
    for at in asked {
        let course = messages.iter().find(|m| m.uid == at.message).and_then(|m| m.claims.get(at.index)).and_then(|c| c.course.clone());
        if let Some(course) = course {
            by_course.entry(course).or_default().push(at.clone());
        }
    }
    for (course, claims) in by_course {
        if !may_call() {
            unsettled.ended = Some("out of time");
            return unsettled;
        }
        let batch = ResolveBatch { candidates: candidates(view, &course), course, claims };
        let answers = match cloudmodel::resolve_mail(client, &batch) {
            Ok(answers) => answers,
            Err(e) if batch_refusal(&e) => {
                unsettled.refused = Some(e.label());
                continue;
            }
            Err(e) => {
                unsettled.ended = Some(e.label());
                return unsettled;
            }
        };
        for answer in answers {
            let claim = messages.iter_mut().find(|m| m.uid == answer.claim.message).and_then(|m| m.claims.get_mut(answer.claim.index));
            if let Some(claim) = claim {
                claim.resolution = Some(Resolution { target_id: answer.target_id });
            }
        }
    }
    unsettled
}

/// The course's items, in the view's order: the notes a stored answer may name (`tasks/` and `archive/`).
fn candidates(view: &VaultView, course: &str) -> Vec<Candidate> {
    view.notes
        .iter()
        .filter(|n| matches!(n.folder.as_str(), "tasks" | "archive") && n.course.as_deref() == Some(course))
        .map(|n| Candidate { id: n.id.clone(), title: n.title.clone(), due: n.due.as_str().map(str::to_string) })
        .collect()
}
