//! The email lane's step machines (spec `2026-09-30-email-forwarding-design.md` §4.2 to §4.5, plan
//! T10.1, spec test 32's step half). Pure data and transitions: no window, no network, no log.
use jiff::civil::date;
use knowlu::mail_steps::{
    backfill_held_for_c3, backfill_since, backfill_window, Action, Mode, Outcome, Paused, Provider, Reason, Routine, Send, Setup, State,
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
    let mut s = setup(Provider::Gmail);
    let ran = drive(&mut s, |_| Outcome::Done);
    assert_eq!(ran, Provider::Gmail.steps(), "a clean run goes through the list in order");
    assert_eq!(s.next_action(), Action::Finished);
    assert!(Provider::Gmail.steps().iter().all(|st| s.state(*st) == State::Done));
}

#[test]
fn a_failed_step_falls_back_to_guided_for_that_step_only_and_later_steps_continue() {
    let mut s = setup(Provider::Gmail);
    let ran = drive(&mut s, |st| if st == Step::Backfill { Outcome::NotFound } else { Outcome::Done });
    assert!(ran.contains(&Step::Backfill), "the canary was done, so the backfill ran: {ran:?}");
    assert_eq!(s.state(Step::Backfill), State::Failed(Reason::NotFound));
    assert_eq!(s.mode(Step::Backfill), Mode::Guided, "the failed step is guided now");
    for st in [Step::SignIn, Step::TurnOn, Step::Canary] {
        assert_eq!(s.mode(st), Mode::Scripted, "{st:?} stays scripted");
    }
    assert_eq!(*ran.last().unwrap(), Step::Finish, "the step after the failure still ran scripted");

    // A failure in the middle: the canary needs forwarding and the backfill needs the canary (Quinn,
    // 2026-10-06), so both hold, and Finish, which closes the window, waits for them (D16's guided path).
    let mut s = setup(Provider::M365);
    let ran = drive(&mut s, |st| if st == Step::Forwarding { Outcome::NotFound } else { Outcome::Done });
    assert_eq!(s.state(Step::Forwarding), State::Failed(Reason::NotFound));
    assert_eq!(s.mode(Step::Forwarding), Mode::Guided);
    assert_eq!(ran, vec![Step::SignIn, Step::StepAside, Step::Forwarding], "nothing after it runs yet");
    for st in [Step::Canary, Step::Backfill, Step::Finish] {
        assert_eq!(s.state(st), State::Waiting, "{st:?} holds");
    }
    assert_eq!(s.next_action(), Action::Wait, "only the student's guided step is left");
    // "I've done this" runs the step's check: a pass makes it done by hand and releases the rest in order.
    s.guided_checked(Step::Forwarding, true);
    assert_eq!(s.state(Step::Forwarding), State::DoneByHand);
    let ran = drive(&mut s, |_| Outcome::Done);
    // D13: the canary runs as script; Microsoft 365's backfill is the guided step, and Finish waits behind it.
    assert_eq!(ran, vec![Step::Canary]);
    assert_eq!((s.state(Step::Backfill), s.mode(Step::Backfill)), (State::NeedsYou(Reason::Guided), Mode::Guided));
    assert_eq!(s.state(Step::Finish), State::Waiting);
}

#[test]
fn the_backfill_waits_for_the_canary_and_a_failed_canary_never_runs_it() {
    // Quinn, 2026-10-06: a backfill run while forwarding or the canary failed brings in old mail while new
    // mail never arrives (D16). It runs only once the canary is done, by script or by hand.
    let is_backfill = |a: &Action| matches!(a, Action::Run { step: Step::Backfill, .. } | Action::Guide { step: Step::Backfill });
    // Forwarding (M365) or turning it on (Gmail) failed, and the student gave up: the backfill never ran.
    for (p, failing) in [(Provider::M365, Step::Forwarding), (Provider::Gmail, Step::TurnOn), (Provider::Gmail, Step::AddAddress)] {
        let mut s = setup(p);
        let ran = drive(&mut s, |st| if st == failing { Outcome::Failed } else { Outcome::Done });
        assert!(!ran.contains(&Step::Backfill), "{p:?} {failing:?}: {ran:?}");
        assert!(!is_backfill(&s.next_action()));
        s.request_stop();
        assert_eq!(s.next_action(), Action::Finished);
        assert_eq!(s.state(Step::Backfill), State::Skipped, "{p:?} {failing:?}");
        assert_eq!(s.summary().mail_not_brought_in_before, Some(date(2026, 6, 4)));
    }
    // Forwarding set by hand, then the canary failed: the backfill is blocked, named, and not run.
    let mut s = setup(Provider::M365);
    drive(&mut s, |st| if st == Step::Forwarding { Outcome::NotFound } else { Outcome::Done });
    s.guided_checked(Step::Forwarding, true);
    let ran = drive(&mut s, |st| if st == Step::Canary { Outcome::Failed } else { Outcome::Done });
    assert!(!ran.contains(&Step::Backfill), "{ran:?}");
    assert_eq!(s.state(Step::Backfill), State::Failed(Reason::Blocked));
    // A guided mailbox: the backfill's guide is shown only once the canary is done by hand.
    let mut g = setup(Provider::Guided);
    assert_eq!(g.next_action(), Action::Guide { step: Step::Forwarding });
    assert_eq!(g.next_action(), Action::Wait);
    g.guided_checked(Step::Forwarding, true);
    assert_eq!(g.next_action(), Action::Guide { step: Step::Canary });
    assert_eq!(g.next_action(), Action::Wait, "no backfill while the canary is the student's");
    g.guided_checked(Step::Canary, true);
    assert_eq!(g.next_action(), Action::Guide { step: Step::Backfill });
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
    let mut full = setup(Provider::Gmail);
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
    // Microsoft 365's backfill is held while C3 is built (D13): see the test of that name below.
    for p in [Provider::Gmail] {
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

#[test]
fn a_sign_in_that_fails_names_its_reason_and_ends_the_setup() {
    // T18c (2026-10-05): a sign-in that did not take must say why on its own row. Every row "skipped"
    // with no reason left a failed setup indistinguishable from one that never began.
    for (out, why) in [(Outcome::NotFound, Reason::NotFound), (Outcome::Failed, Reason::ScriptFailed)] {
        for p in [Provider::M365, Provider::Gmail] {
            let mut s = setup(p);
            let ran = drive(&mut s, |_| out);
            assert_eq!(ran, vec![Step::SignIn], "{p:?} {out:?}: nothing runs after a failed sign-in");
            assert_eq!(s.state(Step::SignIn), State::Failed(why), "{p:?} {out:?}");
            assert_eq!(s.mode(Step::SignIn), Mode::Scripted, "sign-in has no guided version");
            assert_eq!(s.next_action(), Action::Finished);
            for st in &p.steps()[1..] {
                assert_eq!(s.state(*st), State::Skipped, "{p:?} {st:?}");
            }
            assert!(s.summary().done.is_empty());
        }
    }
    // A cancelled sign-in (the window closed, or the sign-in wait ran out) is not a failure: skipped.
    let mut c = setup(Provider::M365);
    drive(&mut c, |_| Outcome::Cancelled);
    assert_eq!(c.state(Step::SignIn), State::Skipped);
}

#[test]
fn a_sign_in_whose_address_the_service_refuses_names_the_refusal_on_its_row() {
    // T18c (2026-10-05): the page signed in, but the service refused its address as the mailbox's hint.
    // Another of the account's mailboxes holding it is `already_connected`; any other refusal is `refused`.
    // Neither reads as a sign-in that did not finish.
    for (out, why) in [(Outcome::AlreadyConnected, Reason::AlreadyConnected), (Outcome::Refused, Reason::Refused)] {
        for p in [Provider::M365, Provider::Gmail] {
            let mut s = setup(p);
            assert_eq!(drive(&mut s, |_| out), vec![Step::SignIn], "{p:?} {out:?}");
            assert_eq!(s.state(Step::SignIn), State::Failed(why), "{p:?} {out:?}");
            assert_eq!(s.mode(Step::SignIn), Mode::Scripted);
            assert_eq!(s.next_action(), Action::Finished);
            for st in &p.steps()[1..] {
                assert_eq!(s.state(*st), State::Skipped, "{p:?} {st:?}");
            }
        }
    }
}

#[test]
fn the_microsoft_backfill_is_the_guided_step_while_c3_is_built_and_is_never_re_armed() {
    // D13 (Quinn, 2026-10-11): the retired DOM backfill never runs for Microsoft 365. The Backfill step is
    // shown as the guided step, after forwarding and the canary have run as script; Gmail is unchanged.
    assert!(backfill_held_for_c3(Provider::M365));
    assert!(!backfill_held_for_c3(Provider::Gmail) && !backfill_held_for_c3(Provider::Guided));
    let mut s = setup(Provider::M365);
    let ran = drive(&mut s, |_| Outcome::Done);
    assert_eq!(ran, [Step::SignIn, Step::StepAside, Step::Forwarding, Step::Canary], "no scripted backfill, and no Finish before the guide");
    assert_eq!((s.state(Step::Backfill), s.mode(Step::Backfill)), (State::NeedsYou(Reason::Guided), Mode::Guided));
    assert_eq!(s.state(Step::Canary), State::Done);
    assert_eq!(s.summary().mail_not_brought_in_before, Some(date(2026, 6, 4)), "the end names the day before which mail was not brought in");
    assert!(!s.rearm_backfill(), "a guided backfill is not failed, so Retry has nothing to re-arm");
    // A blocked backfill (the canary failed) is no different once the canary arrives late: Retry never re-arms it.
    let mut b = setup(Provider::M365);
    drive(&mut b, |st| if st == Step::Canary { Outcome::Failed } else { Outcome::Done });
    assert_eq!(b.state(Step::Backfill), State::Failed(Reason::Blocked));
    b.guided_checked(Step::Canary, true);
    assert!(!b.rearm_backfill());
    assert_eq!((b.state(Step::Backfill), b.mode(Step::Backfill)), (State::Failed(Reason::Blocked), Mode::Guided));
    // A rotate carries no backfill: it keeps its no-op scripted step.
    let mut r = Setup::new_rotate(Provider::M365, today());
    let ran = drive(&mut r, |_| Outcome::Done);
    assert_eq!(ran.last(), Some(&Step::Finish), "{ran:?}");
    // Gmail still runs its backfill as script.
    let mut g = setup(Provider::Gmail);
    assert!(drive(&mut g, |_| Outcome::Done).contains(&Step::Backfill));
}

#[test]
fn a_routine_remembers_when_it_last_read_the_inbox_for_a_report() {
    // In memory only: the instant of the last look at the Inbox, so the next look reads from there.
    let mut r = Routine::new(Provider::Gmail, 9, 0);
    assert_eq!(r.last_look(), None);
    r.looked(1_000);
    assert_eq!(r.last_look(), Some(1_000));
    r.looked(2_500);
    assert_eq!(r.last_look(), Some(2_500));
    // A paused day, a resumed count and a clone all keep it; a refused send does not clear it.
    let mut paused = r.clone().paused();
    paused.send_failed(3_000);
    assert_eq!(paused.last_look(), Some(2_500));
}
