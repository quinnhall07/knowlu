//! Ambiguity asked of the service (spec §4.8 step 3, D12; plan T6.4).
//!
//! A claim the plan leaves ambiguous ([`mailreconcile::resolve::to_ask`]) goes to `mail-resolve` in groups
//! of at most 15 per course, before [`mailreconcile::plan`] runs. The call carries the claims as references
//! (message uid and index: the service holds their text) and the course's items as (id, title, due), the
//! only vault text it sends. Each answer is set on its claim as the stored [`Resolution`] the service
//! also keeps, so the plan reads it as it reads any stored answer: a chosen item is a change to it, and
//! "none" is a notice, never a guess. A claim the service did not answer stays ambiguous: it writes
//! nothing and its message is held, so it is asked again next slot. A course the service refuses for
//! itself (a 4xx other than 401, 402, 403, 408 and 429) is skipped and the next course is still asked; any
//! other failure (session, entitlement, rate, 5xx, no network, no time) ends the asking for the pass.
//! A claim the service never answers is held for [`super::held::GIVE_UP_SLOTS`] slots, then given up on
//! (`held`, PQ14): the pass stores "none" on it and the plan's unmatched notice does the rest.

use std::collections::BTreeMap;

use jiff::civil::Date;

use crate::cloudmodel::{self, Candidate, CloudClient, CloudError, ResolveBatch};
use crate::mailreconcile::{ClaimRef, Message, NoteView, Resolution, VaultView};

/// What [`settle`] could not finish, as closed-set words that name no message text.
#[derive(Default)]
pub(super) struct Unsettled {
    /// What ended the asking early (a failed call that every course would meet, or no time left).
    pub ended: Option<&'static str>,
    /// A course the service refused for itself and was skipped: the courses after it were still asked.
    pub refused: Option<&'static str>,
    /// The claims a call was made about, answered or not: the ones that count as a slot (PQ14). A claim
    /// whose course the asking never reached is not in it.
    pub tried: Vec<ClaimRef>,
}

/// A refusal of one batch (a 4xx that is not about the session, the entitlement, the rate or the clock):
/// the next course is a different request, so it is still asked.
fn batch_refusal(e: &CloudError) -> bool {
    matches!(e, CloudError::Status { code: 400..=499, .. }) && !e.fatal() && !matches!(e, CloudError::Status { code: 408 | 429, .. })
}

/// Ask about `asked` a course at a time (in groups of at most [`MAX_CLAIMS_PER_CALL`] claims), and store
/// each answer on its claim in `messages`. `today` orders a course's candidates when there are too many.
/// `may_call` is the budget's say before each request.
pub(super) fn settle(
    client: &CloudClient,
    view: &VaultView,
    messages: &mut [Message],
    asked: &[ClaimRef],
    today: Date,
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
        // The service takes at most MAX_CLAIMS_PER_CALL claims a call: a course with more is asked in
        // consecutive groups, in the claims' own (plan) order, each offered the same candidates. A
        // refusal or failure of a group is judged as a course's was, and touches only that group's claims.
        let candidates = candidates(view, &course, today);
        for group in claims.chunks(MAX_CLAIMS_PER_CALL) {
            if !may_call() {
                unsettled.ended = Some("out of time");
                return unsettled;
            }
            unsettled.tried.extend(group.iter().cloned());
            let batch = ResolveBatch { candidates: candidates.clone(), course: course.clone(), claims: group.to_vec() };
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
    }
    unsettled
}

/// The most claims one `mail-resolve` call carries: the server's `MAX_CLAIMS`
/// (`cloud/supabase/functions/mail-resolve/handler.ts`), which refuses more with a 400. A test reads that
/// file and fails if the two drift.
pub(super) const MAX_CLAIMS_PER_CALL: usize = 15;

/// The most candidates one `mail-resolve` call carries: the server's `MAX_CANDIDATES` (same file), which
/// refuses more with a 400. A test reads that file and fails if the two drift.
pub(super) const MAX_CANDIDATES_PER_CALL: usize = 200;

/// The course's items the call may name (`tasks/` and `archive/`), at most [`MAX_CANDIDATES_PER_CALL`].
/// In the view's order while they fit. When they do not, the call keeps the ones a student is most likely
/// to mean and drops the rest, deterministically: active notes before archived ones (an archived item is
/// rarely what a mail changes), then the due nearest to `today` first (by days, past or future: the item
/// in play now), undated last, ties by note id. The kept ones are sent in that order.
fn candidates(view: &VaultView, course: &str, today: Date) -> Vec<Candidate> {
    let mut notes: Vec<&NoteView> =
        view.notes.iter().filter(|n| matches!(n.folder.as_str(), "tasks" | "archive") && n.course.as_deref() == Some(course)).collect();
    if notes.len() > MAX_CANDIDATES_PER_CALL {
        let archived = |n: &NoteView| n.folder == "archive" || n.status.as_deref() == Some("archived");
        // `None` (undated, or a due that names no date) sorts after every distance.
        let distance = |n: &NoteView| n.due.as_str().and_then(|d| d.get(..10)).and_then(|d| d.parse::<Date>().ok()).map(|d| (d - today).get_days().unsigned_abs());
        notes.sort_by(|a, b| (archived(a), distance(a).is_none(), distance(a), &a.id).cmp(&(archived(b), distance(b).is_none(), distance(b), &b.id)));
        notes.truncate(MAX_CANDIDATES_PER_CALL);
    }
    notes.into_iter().map(|n| Candidate { id: n.id.clone(), title: n.title.clone(), due: n.due.as_str().map(str::to_string) }).collect()
}

#[cfg(test)]
mod tests {
    use super::{MAX_CANDIDATES_PER_CALL, MAX_CLAIMS_PER_CALL};

    /// The value of `export const NAME = <number>;` in the server's handler source.
    fn server_limit(source: &str, name: &str) -> usize {
        let line = source
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("export const {name} =")))
            .unwrap_or_else(|| panic!("handler.ts exports no {name}"));
        let digits: String = line.split('=').nth(1).unwrap_or("").chars().filter(char::is_ascii_digit).collect();
        digits.parse().unwrap_or_else(|_| panic!("{name} is not a plain number in handler.ts: {line}"))
    }

    /// The engine's two limits are the server's: a drift would make the server refuse a batch with a 400.
    #[test]
    fn the_engines_limits_are_the_servers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../cloud/supabase/functions/mail-resolve/handler.ts");
        let source = std::fs::read_to_string(&path).expect("the mail-resolve handler source is in the repository");
        assert_eq!(MAX_CLAIMS_PER_CALL, server_limit(&source, "MAX_CLAIMS"), "claims per call drifted from handler.ts");
        assert_eq!(MAX_CANDIDATES_PER_CALL, server_limit(&source, "MAX_CANDIDATES"), "candidates per call drifted from handler.ts");
    }
}
