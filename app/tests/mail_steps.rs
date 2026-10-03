//! The email lane's step machines (spec `2026-09-30-email-forwarding-design.md` §4.2 to §4.5, plan
//! T10.1, spec test 32's step half). Pure data and transitions: no window, no network, no log.
use jiff::civil::date;
use knowlu::mail_steps::{
    backfill_since, backfill_window, Action, Mode, Outcome, Paused, Provider, Reason, Routine, Send, Setup, State,
    Step, BACKFILL_DAYS, MAX_PER_DAY, MAX_PER_MINUTE,
};

const DAY: i64 = 86_400;

fn today() -> jiff::civil::Date {
    date(2026, 10, 2)
}

fn setup(p: Provider) -> Setup {
    Setup::new(p, today(), None)
}

/// Runs every scripted step the machine hands out, answering each with `answer`, until it waits or
/// finishes. Returns the steps it was told to run, in order.
fn drive(s: &mut Setup, answer: impl Fn(Step) -> Outcome) -> Vec<Step> {
    let mut ran = Vec::new();
    for _ in 0..64 {
        match s.next_action() {
            Action::Run { step, mode: Mode::Scripted, .. } => {
                ran.push(step);
                s.started(step);
                s.finished(step, answer(step));
            }
            _ => return ran,
        }
    }
    panic!("the machine did not settle in 64 actions");
}

#[test]
fn each_provider_has_its_step_list_in_order() {
    use Step::*;
    assert_eq!(Provider::M365.steps(), &[SignIn, StepAside, Forwarding, Canary, Backfill, Finish]);
    assert_eq!(
        Provider::Gmail.steps(),
        &[SignIn, StepAside, AddAddress, ConfirmAddress, TurnOn, Canary, Backfill, Finish]
    );
    assert_eq!(Provider::Guided.steps(), &[Forwarding, Canary, Backfill]);
    assert_eq!(Provider::from_service("m365"), Provider::M365);
    assert_eq!(Provider::from_service("google"), Provider::Gmail);
    for other in ["other", "", "yahoo", "outlook.com"] {
        assert_eq!(Provider::from_service(other), Provider::Guided, "{other:?} is guided only");
    }
    let mut s = setup(Provider::M365);
    let ran = drive(&mut s, |_| Outcome::Done);
    assert_eq!(ran, Provider::M365.steps(), "a clean run goes through the list in order");
    assert_eq!(s.next_action(), Action::Finished);
    assert!(Provider::M365.steps().iter().all(|st| s.state(*st) == State::Done));
}

#[test]
fn a_failed_step_falls_back_to_guided_for_that_step_only_and_later_steps_continue() {
    let mut s = setup(Provider::Gmail);
    let ran = drive(&mut s, |st| if st == Step::Backfill { Outcome::NotFound } else { Outcome::Done });
    assert_eq!(s.state(Step::Backfill), State::Failed(Reason::NotFound));
    assert_eq!(s.mode(Step::Backfill), Mode::Guided, "the failed step is guided now");
    for st in [Step::SignIn, Step::TurnOn, Step::Canary] {
        assert_eq!(s.mode(st), Mode::Scripted, "{st:?} stays scripted");
    }
    assert_eq!(*ran.last().unwrap(), Step::Finish, "the step after the failure still ran scripted");

    // A failure in the middle: the later steps that do not need it carry on, the one that does waits.
    let mut s = setup(Provider::M365);
    let ran = drive(&mut s, |st| if st == Step::Forwarding { Outcome::NotFound } else { Outcome::Done });
    assert_eq!(s.state(Step::Forwarding), State::Failed(Reason::NotFound));
    assert_eq!(s.mode(Step::Forwarding), Mode::Guided);
    assert_eq!(s.state(Step::Canary), State::Waiting, "the canary needs forwarding, so it holds");
    assert!(ran.contains(&Step::Backfill) && ran.contains(&Step::Finish), "{ran:?}");
    assert_eq!(s.next_action(), Action::Wait, "only the student's guided step is left");
    // "I've done this" runs the step's check: a pass makes it done by hand and releases the canary.
    s.guided_checked(Step::Forwarding, true);
    assert_eq!(s.state(Step::Forwarding), State::DoneByHand);
    let ran = drive(&mut s, |_| Outcome::Done);
    assert_eq!(ran, vec![Step::Canary]);
}

#[test]
fn an_existing_forwarding_address_is_a_needs_you_and_never_replaced() {
    for (p, step) in [(Provider::M365, Step::Forwarding), (Provider::Gmail, Step::AddAddress)] {
        let mut s = setup(p);
        let ran = drive(&mut s, |st| if st == step { Outcome::ForwardingElsewhere } else { Outcome::Done });
        assert_eq!(s.state(step), State::NeedsYou(Reason::ForwardingElsewhere));
        assert!(ran.ends_with(&[step]), "setup stops at the question: {ran:?}");
        assert_eq!(s.next_action(), Action::Wait, "no script runs until the student answers");
        // The student says "stop": nothing is replaced and the step is skipped.
        let mut stopped = s.clone();
        stopped.answer_forwarding_elsewhere(false);
        assert_eq!(stopped.state(step), State::Skipped);
        assert_eq!(stopped.next_action(), Action::Finished);
        // The student says "forward to Knowlu instead": only now does a script run, with the flag.
        s.answer_forwarding_elsewhere(true);
        match s.next_action() {
            Action::Run { step: st, mode: Mode::Scripted, replace_forwarding: true } => assert_eq!(st, step),
            other => panic!("{other:?}"),
        }
    }
    // Every other run carries no permission to replace.
    let mut s = setup(Provider::M365);
    for _ in 0..3 {
        if let Action::Run { step, replace_forwarding, .. } = s.next_action() {
            assert!(!replace_forwarding);
            s.started(step);
            s.finished(step, Outcome::Done);
        }
    }
}

#[test]
fn the_backfill_window_starts_120_days_before_the_setup_day() {
    assert_eq!(BACKFILL_DAYS, 120);
    assert_eq!(backfill_since(date(2026, 10, 2)), date(2026, 6, 4));
    assert_eq!(backfill_since(date(2026, 3, 1)), date(2025, 11, 1), "across a year end and a short February");
    // One constant for every school: the window reads only the setup day.
    assert_eq!(backfill_window(None, date(2026, 10, 2)), date(2026, 6, 4));
    assert_eq!(setup(Provider::M365).backfill_since(), date(2026, 6, 4));
    assert_eq!(setup(Provider::Gmail).backfill_since(), date(2026, 6, 4));
    assert_eq!(setup(Provider::Guided).backfill_since(), date(2026, 6, 4));
}

#[test]
fn a_resumed_backfill_keeps_its_first_window_start() {
    let first = backfill_since(date(2026, 10, 2));
    // Thirty days later the service still holds the first start; the resume reads it.
    let later = date(2026, 11, 1);
    assert_eq!(backfill_window(Some(first), later), first);
    assert_ne!(backfill_since(later), first, "a recomputed window would have moved");
    let resumed = Setup::new(Provider::M365, later, Some(first));
    assert_eq!(resumed.backfill_since(), first);
    // The summary of a stopped run names the same first start.
    let mut s = resumed;
    s.request_stop();
    assert_eq!(s.summary().mail_not_brought_in_before, Some(first));
}

#[test]
fn the_pace_is_at_most_10_a_minute_and_600_a_mailbox_a_day() {
    assert_eq!((MAX_PER_MINUTE, MAX_PER_DAY), (10, 600));
    let t0 = 5 * DAY + 3_600;
    let mut r = Routine::new(Provider::M365, t0.div_euclid(DAY), 0);
    for i in 0..10 {
        assert_eq!(r.next(t0 + i), Send::Now, "send {i}");
        r.sent(t0 + i);
    }
    // The eleventh waits until the first send is a minute old.
    assert_eq!(r.next(t0 + 10), Send::WaitUntil(t0 + 60));
    assert_eq!(r.next(t0 + 59), Send::WaitUntil(t0 + 60));
    assert_eq!(r.next(t0 + 60), Send::Now, "one slot is free once the first send ages out");
    r.sent(t0 + 60);
    assert_eq!(r.next(t0 + 60), Send::WaitUntil(t0 + 61), "ten sends are again inside the minute");

    // 600 in a UTC day, then nothing until the next UTC day.
    let start = 9 * DAY;
    let mut r = Routine::new(Provider::Gmail, 9, 599);
    assert_eq!(r.next(start + 100), Send::Now);
    r.sent(start + 100);
    assert_eq!(r.next(start + 1_000), Send::DayLimit { next_day_at: 10 * DAY });
    assert_eq!(r.next(10 * DAY - 1), Send::DayLimit { next_day_at: 10 * DAY });
    assert_eq!(r.next(10 * DAY), Send::Now, "the new UTC day starts at zero");
    // A resume on the same day counts what was already sent.
    let mut resumed = Routine::new(Provider::Gmail, 9, 600);
    assert!(matches!(resumed.next(start + 5), Send::DayLimit { .. }));
}

#[test]
fn the_first_send_failure_stops_the_routine_with_its_named_outcome() {
    let t0 = 5 * DAY + 100;
    let mut r = Routine::new(Provider::M365, 5, 0);
    assert_eq!(r.next(t0), Send::Now);
    r.sent(t0);
    r.send_failed(t0 + 1);
    let paused = Paused::new(Provider::M365);
    assert_eq!(r.next(t0 + 2), Send::Paused(paused));
    assert_eq!(paused.line(), "mail: forwarding paused \u{2014} Microsoft refused a send");
    assert_eq!(Paused::new(Provider::Gmail).line(), "mail: forwarding paused \u{2014} Gmail refused a send");
    // Nothing retries until the next UTC day.
    assert_eq!(r.next(t0 + 3_000), Send::Paused(paused));
    assert_eq!(r.next(5 * DAY + DAY - 1), Send::Paused(paused));
    assert_eq!(r.next(6 * DAY), Send::Now);
}

#[test]
fn nothing_is_retried_in_a_loop() {
    // A step that fails is handed out once, however often the machine is asked.
    let mut s = setup(Provider::Gmail);
    let mut runs = std::collections::BTreeMap::<String, u32>::new();
    for _ in 0..200 {
        match s.next_action() {
            Action::Run { step, mode: Mode::Scripted, .. } => {
                *runs.entry(format!("{step:?}")).or_default() += 1;
                s.started(step);
                s.finished(step, if step == Step::SignIn { Outcome::Done } else { Outcome::Failed });
            }
            Action::Run { mode: Mode::Guided, .. } => panic!("a guided step is never run as script"),
            _ => break,
        }
    }
    assert!(runs.values().all(|n| *n == 1), "{runs:?}");
    // A guided check that fails leaves the step failed: the machine does not run it again on its own.
    let before = s.clone();
    s.guided_checked(Step::AddAddress, false);
    assert_eq!(s.state(Step::AddAddress), before.state(Step::AddAddress));
    assert!(!matches!(s.next_action(), Action::Run { step: Step::AddAddress, .. }));
    // A cancelled sign-in ends the setup with nothing changed: no step runs after it.
    let mut c = setup(Provider::M365);
    let ran = drive(&mut c, |_| Outcome::Cancelled);
    assert_eq!(ran, vec![Step::SignIn]);
    assert_eq!(c.next_action(), Action::Finished);
    assert!(c.summary().done.is_empty());
    // And the send routine never retries a failed send in the same day.
    let mut r = Routine::new(Provider::M365, 0, 0);
    r.send_failed(10);
    for t in (11..2_000).step_by(37) {
        assert!(matches!(r.next(t), Send::Paused(_)));
    }
}

#[test]
fn stop_after_the_current_step_says_what_was_done() {
    let mut s = setup(Provider::M365);
    // SignIn and StepAside done; Forwarding is running when the student presses Stop.
    for _ in 0..3 {
        if let Action::Run { step, .. } = s.next_action() {
            s.started(step);
            if step != Step::Forwarding {
                s.finished(step, Outcome::Done);
            }
        }
    }
    assert_eq!(s.state(Step::Forwarding), State::Working);
    s.request_stop();
    // The current step is allowed to finish; no later step starts.
    assert_eq!(s.next_action(), Action::Wait, "the running step is not cut off");
    s.finished(Step::Forwarding, Outcome::Done);
    assert_eq!(s.next_action(), Action::Finished);
    for st in [Step::Canary, Step::Backfill, Step::Finish] {
        assert_eq!(s.state(st), State::Skipped);
    }
    let sum = s.summary();
    assert_eq!(sum.done, vec![Step::SignIn, Step::StepAside, Step::Forwarding]);
    assert_eq!(sum.left, vec![Step::Canary, Step::Backfill, Step::Finish]);
    assert_eq!(sum.mail_not_brought_in_before, Some(date(2026, 6, 4)), "forwarding is on; mail before this was not brought in");

    // Had the backfill finished, nothing is left out.
    let mut full = setup(Provider::M365);
    drive(&mut full, |_| Outcome::Done);
    full.request_stop();
    let sum = full.summary();
    assert!(sum.left.is_empty());
    assert_eq!(sum.mail_not_brought_in_before, None);
}

#[test]
fn a_failed_canary_leaves_only_guided_steps() {
    // D16: a school that blocks forwarding fails the canary; no scripted backfill follows, for either
    // provider. The backfill is guided, named as blocked, and the cleanup Finish still runs.
    for p in [Provider::M365, Provider::Gmail] {
        let mut s = setup(p);
        let ran = drive(&mut s, |st| if st == Step::Canary { Outcome::Failed } else { Outcome::Done });
        assert!(!ran.contains(&Step::Backfill), "{p:?} must not run a scripted backfill: {ran:?}");
        assert_eq!(s.state(Step::Canary), State::Failed(Reason::ScriptFailed));
        assert_eq!(s.state(Step::Backfill), State::Failed(Reason::Blocked));
        assert_eq!(s.mode(Step::Backfill), Mode::Guided);
        assert_eq!(*ran.last().unwrap(), Step::Finish, "Finish only closes the window and still runs");
        assert_eq!(s.summary().mail_not_brought_in_before, Some(date(2026, 6, 4)));
        // The student's own backfill count passes: done by hand.
        s.guided_checked(Step::Backfill, true);
        assert_eq!(s.state(Step::Backfill), State::DoneByHand);
    }
}

#[test]
fn a_retry_re_arms_a_backfill_only_once_the_canary_has_arrived() {
    for p in [Provider::M365, Provider::Gmail] {
        let mut s = setup(p);
        drive(&mut s, |st| if st == Step::Canary { Outcome::Failed } else { Outcome::Done });
        assert!(!s.rearm_backfill(), "{p:?}: the canary is still failed");
        s.guided_checked(Step::Canary, true);
        assert!(s.rearm_backfill());
        assert_eq!((s.state(Step::Backfill), s.mode(Step::Backfill)), (State::Waiting, Mode::Scripted));
        assert!(matches!(s.next_action(), Action::Run { step: Step::Backfill, .. }), "{p:?} runs it now");
    }
    // A guided mailbox's backfill is the student's; nothing re-arms it.
    let mut g = setup(Provider::Guided);
    assert!(!g.rearm_backfill());
    // A pace carries a pause and a count.
    let r = Routine::new(Provider::M365, 9, 41).paused();
    assert_eq!((r.day(), r.sent_today(), r.is_paused()), (9, 41, true));
}
