//! The mail pass (spec §4.8 steps 1 and 4 to 7, §4.10; plan T6.3): pull, view, plan, apply, ack, line.
//!
//! **Pull.** Live mail takes up to [`PULL_ROUNDS`] rounds a slot; a closed batch (a `backfill` message in
//! the last answer) drains past them, until `budget` is spent, which is checked **before** each round as
//! `pull_gmail` checks it. The ack of a round goes with the next pull, and the last one in a pull of its
//! own, so a crash costs a repeat, never a lost message.
//!
//! **Apply.** Each round is viewed ([`super::view`]), planned by `mailreconcile::plan` (the only place
//! card or direct is decided) and written through [`Writer`], in plan order. Before it plans, the ambiguous
//! claims are asked of `mail-resolve`, one call per course ([`super::asks`], T6.4). A message is acknowledged and
//! recorded in `state/ingest-seen.md` (under the fixed title `(email)`, never a mail-derived one: plan
//! §10, R11) only when every claim of it was written or deliberately skipped. A write that failed, or a
//! claim still ambiguous with a course (unanswered, or asking failed), holds its message: nothing is
//! acknowledged, so it comes back, and a claim written before is `AlreadyApplied` when it does. The server
//! returns an undelivered claim on every later pull, so a held message that comes back in the same pass is
//! skipped (not planned, written or counted again), and a page of nothing but held messages ends the pass:
//! it would be returned unchanged until acked. A claim `mail-resolve` never answers is held for at most
//! three slots ([`super::held`], PQ14): at the third the pass stores "none" on it, so the plan opens the
//! unmatched notice, and the message is acked and the drain goes on.
//!
//! **The line.** `mail: N new, M changed, K proposed, J notices, P past, S superseded, U unmatched`
//! (§4.8 step 7), printed when a mailbox is connected or a message arrived, and never otherwise. A
//! residual `NotAmendable` counts in `U` (PQ7). A failure or an unreadable row is counted on the end of
//! the line, as `pull_gmail`'s summary does, and never named.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{Duration, Instant};

use jiff::civil::Date;

use crate::cloudmodel::{self, CloudClient, MailPull, MailboxReport, CALL_TIMEOUT};
use crate::enrich::{Options, PULL_ROUNDS};
use crate::ingest;
use crate::journal::Journal;
use crate::mailreconcile::{self, resolve, Action, Claim, ClaimRef, Message, NoticeAbout, Phase, Resolution, SkipReason};

use super::held::{Held, GIVE_UP_SLOTS};
use super::{asks, heads, view, Source, Why, Wrote, Writer};

/// The pass over the vault's own clock (`config/ingest.yaml`'s zone, else the machine's), which is the
/// clock `commitments::vault_day` counts the vault's days in.
pub fn pull_mail(vault: &Path, client: &CloudClient, opts: &Options<'_>, budget: Duration) -> Vec<String> {
    let today = jiff::Timestamp::now().to_zoned(crate::cli::vault_zone(vault)).date();
    pull_mail_on(vault, client, opts, budget, today)
}

/// [`pull_mail`] on a given `today`, so a test can put a vault on its first or second day.
pub fn pull_mail_on(vault: &Path, client: &CloudClient, opts: &Options<'_>, budget: Duration, today: Date) -> Vec<String> {
    let started = Instant::now();
    let mut journal = Journal::new(vault);
    let mut run = Run {
        vault,
        client,
        opts,
        today,
        started,
        budget,
        tally: Tally::default(),
        boxes: BTreeMap::new(),
        connected: false,
        held: BTreeSet::new(),
        asking_failed: None,
        asking_refused: None,
        slots: Held::load(vault),
        struck: BTreeSet::new(),
        gave_up: 0,
        marked: Vec::new(),
        waiting: false,
    };
    let (mut ack, mut round, mut batch, mut more) = (Vec::<String>::new(), 0usize, false, false);
    let mut failure: Option<String> = None;
    loop {
        // Live mail takes `PULL_ROUNDS`; only a closed batch is drained past them.
        if round >= PULL_ROUNDS && !batch {
            break;
        }
        if started.elapsed() + CALL_TIMEOUT > budget {
            break;
        }
        let pulled = match cloudmodel::pull_mail_claims(client, &ack) {
            Ok(pulled) => pulled,
            Err(e) => {
                failure = Some(format!("mail: skipped ({e})"));
                break;
            }
        };
        run.acked(&ack);
        ack.clear();
        round += 1;
        more = pulled.more;
        batch = pulled.messages.iter().any(|m| m.phase == Phase::Backfill);
        let stalled = run.round(&pulled, &mut journal, &mut ack);
        // A page of nothing but held messages comes back unchanged until it is acked: no progress.
        if !more || stalled {
            break;
        }
    }
    if !ack.is_empty() && failure.is_none() {
        match cloudmodel::pull_mail_claims(client, &ack) {
            Ok(_) => run.acked(&ack),
            Err(e) => {
                failure = Some(format!("mail: {} written but not acknowledged ({e}); they will come back next slot", ack.len()));
            }
        }
    }
    let mut lines = Vec::new();
    if run.connected {
        lines.push(run.tally.line());
    }
    let boxes: Vec<&MailboxReport> = run.boxes.values().collect();
    let held = mailreconcile::proposal(today, crate::commitments::vault_day(vault, today));
    let mut writer = Writer::new(vault, opts.via, opts.run_id, &mut journal, today, held);
    lines.extend(heads::keep(&mut writer, vault, &boxes, &today.to_string()));
    // Only when a claim still waits: one given up on is a notice, not "left for the next slot".
    if let Some(why) = run.asking_failed.or(run.asking_refused).filter(|_| run.waiting) {
        lines.push(format!("mail: could not ask which item ({why}); left for the next slot"));
    }
    if run.gave_up > 0 {
        let s = if run.gave_up == 1 { "" } else { "s" };
        lines.push(format!("mail: gave up asking which item for {} claim{s} after {GIVE_UP_SLOTS} slots; each is an unmatched notice", run.gave_up));
    }
    if more && failure.is_none() {
        lines.push("mail: more is waiting, left for the next slot".to_string());
    }
    lines.extend(failure);
    lines
}

/// What one slot's rounds share.
struct Run<'a> {
    vault: &'a Path,
    client: &'a CloudClient,
    opts: &'a Options<'a>,
    today: Date,
    started: Instant,
    budget: Duration,
    tally: Tally,
    /// The latest report of each mailbox, by `address_id`.
    boxes: BTreeMap<String, MailboxReport>,
    /// A mailbox was reported or a message arrived: the line prints.
    connected: bool,
    /// The uids held unacked this pass. The server returns an undelivered claim on every later pull, so a
    /// held message that comes back is neither planned, written nor counted again (a notice written for
    /// it would otherwise be closed and reopened, and counted in `J` each round).
    held: BTreeSet<String>,
    /// What ended the asking of `mail-resolve` (a failed call, no time left): the rest of the pass does not
    /// ask again, since the next call would fail the same way, and the claims wait for the next slot.
    asking_failed: Option<&'static str>,
    /// A course the service refused for itself was skipped: later courses were still asked, and its
    /// claims wait for the next slot. Named like `asking_failed`.
    asking_refused: Option<&'static str>,
    /// Slots each unanswered claim has been held, on disk (PQ14, [`super::held`]).
    slots: Held,
    /// The claims already counted as a slot this pass: a claim is counted once however many rounds run.
    struck: BTreeSet<String>,
    /// Claims given up on this pass (the line names them).
    gave_up: usize,
    /// Given-up claims (message uid, claim uid) whose marker in `slots` stays until the message is acked.
    marked: Vec<(String, String)>,
    /// Some claim still waits on `mail-resolve` after this pass's asking.
    waiting: bool,
}

impl Run<'_> {
    /// Apply one answer: its mailboxes noted, its messages planned, written and (when whole) acked.
    /// Returns `true` when the answer held nothing but messages this pass already holds (no progress).
    fn round(&mut self, pulled: &MailPull, journal: &mut Journal, ack: &mut Vec<String>) -> bool {
        self.connected |= !pulled.mailboxes.is_empty() || !pulled.messages.is_empty();
        self.tally.unreadable += pulled.unreadable;
        for mailbox in &pulled.mailboxes {
            self.boxes.insert(mailbox.address_id.clone(), mailbox.clone());
        }
        // A message already recorded is acknowledged again and planned no more (an ack that never arrived).
        let stalled = !pulled.messages.is_empty() && pulled.messages.iter().all(|m| self.held.contains(&m.uid));
        let seen = ingest::load_seen(self.vault);
        let (done, fresh): (Vec<&Message>, Vec<&Message>) = pulled.messages.iter().partition(|m| seen.contains(&m.uid));
        // A message this pass already holds is skipped whole: its claims were planned and written once.
        let fresh: Vec<&Message> = fresh.into_iter().filter(|m| !self.held.contains(&m.uid)).collect();
        let mut acked: BTreeSet<String> = BTreeSet::new();
        for message in done {
            if acked.insert(message.uid.clone()) {
                ack.push(message.uid.clone());
            }
            // Recorded, so whole: no claim of it is counted or marked any more.
            for index in 0..message.claims.len() {
                self.slots.clear(&ClaimRef { message: message.uid.clone(), index }.uid());
            }
        }
        self.slots.save();
        if fresh.is_empty() {
            return stalled;
        }
        let fresh: Vec<Message> = fresh.into_iter().cloned().collect();
        let held = self.write_plan(&fresh, pulled, journal);
        self.held.extend(held.iter().cloned());
        let stamp = self.today.to_string();
        for message in &fresh {
            if held.contains(&message.uid) || !acked.insert(message.uid.clone()) {
                continue;
            }
            match ingest::record_seen(self.vault, &message.uid, "(email)", &stamp) {
                Ok(()) => ack.push(message.uid.clone()),
                // Not acknowledged: it comes back, and every claim of it is already applied.
                Err(_) => {
                    self.tally.failed += 1;
                    self.held.insert(message.uid.clone());
                }
            }
        }
        false
    }

    /// The messages in `ack` were acknowledged: the markers of the claims given up on in them go.
    fn acked(&mut self, ack: &[String]) {
        let marked = std::mem::take(&mut self.marked);
        let (gone, kept): (Vec<_>, Vec<_>) = marked.into_iter().partition(|(message, _)| ack.contains(message));
        self.marked = kept;
        for (_, uid) in gone {
            self.slots.clear(&uid);
        }
        self.slots.save();
    }

    /// Store "none" on a claim given up on (the plan then opens D12's unmatched notice and the message
    /// is acked). Its marker stays on disk until the message is acknowledged.
    fn give_up(&mut self, message: &str, index: usize, claim: &mut Claim) {
        claim.resolution = Some(Resolution { target_id: None });
        self.marked.push((message.to_string(), ClaimRef { message: message.to_string(), index }.uid()));
        self.gave_up += 1;
    }

    /// PQ14: count one slot against each claim that was `tried` and is still unanswered, and give up on
    /// the one that reaches [`GIVE_UP_SLOTS`]. An answered claim, and one no longer asked, drops its
    /// count. A claim `asked` but not `tried` (the asking ended before it) is held without a count. A
    /// claim already at the cap (given up on, its message not yet acked) is given up on again, unasked.
    fn count_slots(&mut self, fresh: &mut [Message], asked: &[ClaimRef], tried: &[ClaimRef]) {
        for message in fresh.iter_mut() {
            for index in 0..message.claims.len() {
                let at = ClaimRef { message: message.uid.clone(), index };
                let uid = at.uid();
                let claim = &mut message.claims[index];
                let capped = self.slots.given_up(&uid);
                if !asked.contains(&at) || (claim.resolution.is_some() && !capped) {
                    self.slots.clear(&uid);
                } else if claim.resolution.is_none() && capped && !tried.contains(&at) {
                    self.give_up(&message.uid, index, claim);
                } else if tried.contains(&at) && self.struck.insert(uid.clone()) {
                    if self.slots.strike(&uid) >= GIVE_UP_SLOTS {
                        self.give_up(&message.uid, index, claim);
                    }
                }
            }
        }
        self.slots.save();
    }

    /// Plan `fresh` against the vault now and write the actions in order. Returns the uids of the
    /// messages that must not be acknowledged yet.
    fn write_plan(&mut self, fresh: &[Message], pulled: &MailPull, journal: &mut Journal) -> BTreeSet<String> {
        let view = view::build(self.vault, journal);
        let vault_day = crate::commitments::vault_day(self.vault, self.today);
        // §4.8 step 3: the ambiguous claims are asked, one call per course, and the answers stored on them
        // before the plan reads them. What stays ambiguous (unanswered, or asking failed) waits.
        let mut fresh = fresh.to_vec();
        let asked: Vec<ClaimRef> = resolve::to_ask(&view, &fresh, self.today, vault_day).into_iter().map(|(at, _)| at).collect();
        let mut tried: Vec<ClaimRef> = Vec::new();
        let to_settle: Vec<ClaimRef> = asked.iter().filter(|at| !self.slots.given_up(&at.uid())).cloned().collect();
        if !to_settle.is_empty() && self.asking_failed.is_none() {
            let (started, budget) = (self.started, self.budget);
            let unsettled = asks::settle(self.client, &view, &mut fresh, &to_settle, || started.elapsed() + CALL_TIMEOUT <= budget);
            self.asking_failed = unsettled.ended;
            self.asking_refused = self.asking_refused.or(unsettled.refused);
            tried = unsettled.tried;
        }
        self.count_slots(&mut fresh, &asked, &tried);
        let fresh = &fresh[..];
        let waiting: BTreeSet<ClaimRef> =
            resolve::to_ask(&view, fresh, self.today, vault_day).into_iter().map(|(at, _)| at).collect();
        self.waiting |= !waiting.is_empty();
        let actions = mailreconcile::plan(&view, fresh, self.today, vault_day);
        let proposal = mailreconcile::proposal(self.today, vault_day);
        let mut writer = Writer::new(self.vault, self.opts.via, self.opts.run_id, journal, self.today, proposal);
        let mut held: BTreeSet<String> = BTreeSet::new();
        for action in &actions {
            let at = action.claim();
            let Some(message) = fresh.iter().find(|m| m.uid == at.message) else { continue };
            let src = Source {
                message,
                claim: &message.claims[at.index],
                judgment_id: pulled.judgment_ids.get(&message.uid).map(String::as_str),
            };
            let waits = waiting.contains(at);
            match writer.apply(action, &src) {
                // The note this claim was to land on was never written: it waits for its own message.
                Ok(Wrote::Nothing(Why::NoPlannedNote)) | Err(_) => {
                    self.tally.failed += 1;
                    held.insert(message.uid.clone());
                }
                Ok(wrote) => self.tally.count(action, &wrote, waits),
            }
            if waits {
                held.insert(message.uid.clone());
            }
        }
        held
    }
}

/// §4.8 step 7's seven counts, and the two this pass adds when it has to.
#[derive(Default)]
struct Tally {
    new: usize,
    changed: usize,
    proposed: usize,
    notices: usize,
    past: usize,
    superseded: usize,
    unmatched: usize,
    failed: usize,
    unreadable: u64,
}

impl Tally {
    /// Count one written action. `waits`: its claim is ambiguous and waits on `mail-resolve`, so the
    /// skip it plans is not yet "unmatched".
    fn count(&mut self, action: &Action, wrote: &Wrote, waits: bool) {
        match wrote {
            Wrote::Note(_) => self.new += 1,
            Wrote::Set(_) => self.changed += 1,
            // A receipt that asked (a card), or a direct set a student's `set` turned into a card.
            Wrote::Card(_) | Wrote::Receipt(_) => self.proposed += 1,
            Wrote::Notice(_) => match action {
                Action::Notice { about: NoticeAbout::CourseInfo, .. } => self.notices += 1,
                Action::Notice { about: NoticeAbout::Unmatched, .. } => self.unmatched += 1,
                _ => {}
            },
            Wrote::Nothing(Why::Skipped(SkipReason::Past)) => self.past += 1,
            Wrote::Nothing(Why::Skipped(SkipReason::Superseded)) => self.superseded += 1,
            // PQ7: a residual `NotAmendable`, from the planner or the writer's own check, counts in `U`.
            Wrote::Nothing(Why::Skipped(SkipReason::NotAmendable) | Why::NotAmendable) => self.unmatched += 1,
            Wrote::Nothing(Why::Skipped(SkipReason::Unmatched)) if !waits => self.unmatched += 1,
            Wrote::Nothing(_) => {}
        }
    }

    fn line(&self) -> String {
        let mut line = format!(
            "mail: {} new, {} changed, {} proposed, {} notices, {} past, {} superseded, {} unmatched",
            self.new, self.changed, self.proposed, self.notices, self.past, self.superseded, self.unmatched
        );
        if self.failed > 0 {
            line.push_str(&format!(", {} not written", self.failed));
        }
        if self.unreadable > 0 {
            line.push_str(&format!(", {} unreadable", self.unreadable));
        }
        line
    }
}
