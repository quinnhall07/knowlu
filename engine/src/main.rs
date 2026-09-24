//! `knowlu-engine` CLI.
//!
//! Subcommands mirror the Python `python -m engine.*` entry points one-for-one, because the
//! cloud routine's command surface is the contract the port eventually has to satisfy. See the
//! rewrite spec section 4 for the mapping.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use knowlu_engine::info::{self, InfoCommand};
use knowlu_engine::issues::{self, IssueCommand};
use knowlu_engine::write::{self, WriteCommand};
use knowlu_engine::{cli, coursework, enrich, ingest, journal, runs};

#[derive(Parser)]
#[command(name = "knowlu-engine", version, about = "Deterministic personal operations engine")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Rank tasks, run the passes, and render state/today.md. Ports `python -m engine.cli`.
    Rank {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Pin the run date (YYYY-MM-DD). Without it, the system date is used.
        #[arg(long)]
        today: Option<String>,
        #[arg(long, default_value = "manual", value_parser = ["manual", "local", "cloud"])]
        runner: String,
        /// Reuse a run id the caller already started.
        #[arg(long = "run-id")]
        run_id: Option<String>,
    },
    /// Print the console's read model as JSON. Never writes. Ports nothing — S2's read model was
    /// designed in Python and built here first (console spec §4.6).
    Surface {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// today | overdue | week | later | all | decisions | good-to-know | issues | runs
        #[arg(long, default_value = "today")]
        view: String,
        /// Pin the date (YYYY-MM-DD). Without it, the system date is used.
        #[arg(long)]
        today: Option<String>,
        /// Pin the clock (YYYY-MM-DDTHH:MM, vault time zone). Without it, now.
        #[arg(long)]
        now: Option<String>,
        /// The per-device "last looked" stamp for the delta line.
        #[arg(long = "seen-at")]
        seen_at: Option<String>,
        /// Override the embedded build SHA (the frozen references pin this to "pinned").
        #[arg(long = "build-sha")]
        build_sha: Option<String>,
        /// Preview a planning window (a flow sequence shaped like the planning-day note's
        /// `window`) on the today view, with `moved` against the current window. Writes nothing.
        #[arg(long)]
        window: Option<String>,
    },
    /// Fetch, refresh the series file, and print the current commitment proposals — the
    /// phase-2 confirm screen's data source (spec §5.1, R14). Writes no note, no card and no
    /// journal record; `state/calendar.md` is untouched. Always exits 0. With `--confirm`
    /// (phase-2 spec §3) it fetches nothing: it writes the screen's answers from a JSON file,
    /// prints `{created, declined, warnings, window}`, and exits 2 on unreadable input or an
    /// invalid window, having written nothing.
    Commitments {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Pin the run date (YYYY-MM-DD). Without it, the vault's zone and the system date.
        #[arg(long)]
        today: Option<String>,
        #[arg(long)]
        json: bool,
        /// `{"mine": [{"source_uid", "level"}], "not_mine": [...], "window": "<flow sequence>"}`.
        #[arg(long)]
        confirm: Option<PathBuf>,
        #[arg(long, default_value = "quinn")]
        actor: String,
        #[arg(long, default_value = "dashboard", value_parser = journal::VIAS)]
        via: String,
    },
    /// Sync zyBooks + VHL coursework into tasks/. Ports `python -m engine.coursework`.
    ///
    /// Always exits 0: coursework must never fail the run, because the rank matters more than
    /// the fetch. Failures reach `state/runner-log.md` instead.
    Coursework {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Report what would change without writing a note, a ledger line or a journal record.
        #[arg(long = "dry-run")]
        dry_run: bool,
        /// Without these the runner's writes journal as `via: cli, run_id: null` --
        /// indistinguishable from someone typing the command by hand.
        #[arg(long, default_value = "cli", value_parser = journal::VIAS)]
        via: String,
        #[arg(long = "run-id")]
        run_id: Option<String>,
    },
    /// What the coursework sources can see, without writing anything: the zyBooks books and the
    /// VHL sections this account reaches, and — with `--vault` — whether that vault's
    /// `config/ingest.yaml` already places each one. One JSON object on stdout; always exit 0.
    CourseworkDiscover {
        /// Optional: with it, the credential targets come from the vault's own config and
        /// `mapped` is computed (`enabled` is not read: discovery cares what an account can see,
        /// not whether the vault currently syncs it). Without it (the wizard, whose vault does
        /// not exist yet) the targets come from the two flags and `mapped` is always false.
        #[arg(long)]
        vault: Option<PathBuf>,
        /// The Windows Credential Manager target to sign in to zyBooks with; overrides the
        /// vault's `coursework.zybooks.credential_target`.
        #[arg(long = "zybooks-target")]
        zybooks_target: Option<String>,
        /// The Windows Credential Manager target to sign in to VHL with; overrides the vault's
        /// `coursework.vhl.credential_target`.
        #[arg(long = "vhl-target")]
        vhl_target: Option<String>,
    },
    /// Sync the LMS .ics feed into tasks/. Ports `python -m engine.ingest`.
    Ingest {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Without these the runner's writes journal as `via: cli, run_id: null` --
        /// indistinguishable from someone typing the command by hand.
        #[arg(long, default_value = "cli", value_parser = journal::VIAS)]
        via: String,
        #[arg(long = "run-id")]
        run_id: Option<String>,
    },
    /// Enrich tasks flagged `needs_enrichment: true` using the local model, if one is installed.
    ///
    /// Always exits 0: no runtime and no model are normal outcomes (Knowlu spec §5.3), and a
    /// non-zero exit here would put the app's scheduler into retry backoff and paint the tray
    /// amber for a machine that has simply not downloaded a model.
    Judge {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        /// Without these the run's writes journal as `via: cli, run_id: null` --
        /// indistinguishable from someone typing the command by hand.
        #[arg(long, default_value = "cli", value_parser = journal::VIAS)]
        via: String,
        #[arg(long = "run-id")]
        run_id: Option<String>,
        /// The llama.cpp runtime binary. Without it: KNOWLU_RUNTIME, else a sibling of this exe.
        #[arg(long)]
        runtime: Option<PathBuf>,
        /// The .gguf model file. Without it: KNOWLU_MODEL, else "model not installed".
        #[arg(long)]
        model: Option<PathBuf>,
        /// Where judgments are logged. **Never inside the vault** (spec §5.6) -- the app passes its
        /// profile's app-data folder; a hand-typed run without it logs nothing.
        #[arg(long = "log-dir")]
        log_dir: Option<PathBuf>,
        /// How many items one run judges. The rest wait for the next slot.
        ///
        /// m3 (Task 7 fix round 1): `0` would print `0 item(s) ... 1 left for the next slot` and
        /// exit 0, forever, if anything ever passed it -- a run that can never make progress. A
        /// clap usage error for that is a different thing from a slot failure, and every normal
        /// path here still exits 0.
        #[arg(long, default_value_t = enrich::DEFAULT_LIMIT, value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..))]
        limit: usize,
    },
    /// Run records. Ports `python -m engine.runs`.
    Runs {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        #[command(subcommand)]
        command: RunsCommand,
    },
    /// Info items: things to know, opened and closed by key. Ports `python -m engine.info`.
    Info {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        #[arg(long, default_value = "quinn")]
        actor: String,
        #[arg(long, default_value = "cli")]
        via: String,
        #[command(subcommand)]
        command: InfoArgs,
    },
    /// Issue notes: flag an AI judgment. Ports `python -m engine.issues`.
    Issues {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        #[arg(long, default_value = "quinn")]
        actor: String,
        #[arg(long, default_value = "cli")]
        via: String,
        #[command(subcommand)]
        command: IssueArgs,
    },
    /// Edit a note and journal the change. Ports `python -m engine.write`.
    Write {
        #[arg(long, default_value = ".")]
        vault: PathBuf,
        #[arg(long, default_value = "quinn")]
        actor: String,
        #[arg(long, default_value = "cli", value_parser = journal::VIAS)]
        via: String,
        #[arg(long = "run-id")]
        run_id: Option<String>,
        #[command(subcommand)]
        command: WriteArgs,
    },
}

#[derive(Subcommand)]
enum WriteArgs {
    /// Set frontmatter fields: `field=literal`, one or more.
    Set {
        /// An opaque id or a vault-relative path.
        target: String,
        #[arg(required = true, num_args = 1..)]
        pairs: Vec<String>,
        /// Apply the judge-once rule and write a provenance block.
        #[arg(long)]
        judged: bool,
        /// File a `kind: amend` proposal instead of skipping a field Quinn set.
        #[arg(long)]
        propose: bool,
        /// JSON mapping recorded in the judgment block.
        #[arg(long)]
        inputs: Option<String>,
        /// JSON mapping recorded on each field's journal record.
        #[arg(long)]
        evidence: Option<String>,
    },
    /// Create a note from a file's text; stamps an id and journals the whole note.
    Create {
        rel_path: String,
        #[arg(long = "from-file", required = true)]
        from_file: PathBuf,
    },
    /// Settle a note into `archive/`. Nothing is ever unlinked.
    Delete {
        target: String,
    },
    /// Relocate a note inside the vault.
    Move {
        target: String,
        new_rel: String,
    },
    /// Append one `> line` to a note's body, idempotently.
    AppendBody {
        target: String,
        #[arg(long, required = true)]
        line: String,
    },
}

#[derive(Subcommand)]
enum InfoArgs {
    /// Open an item in `info/`.
    Open {
        #[arg(long, required = true)]
        title: String,
        #[arg(long, default_value = "notice", value_parser = info::KINDS)]
        kind: String,
        #[arg(long, default_value = "")]
        body: String,
        #[arg(long = "opened-by", default_value = "quinn")]
        opened_by: String,
        /// The exact key a later close signal will carry.
        #[arg(long = "close-key")]
        close_key: Option<String>,
        /// YYYY-MM-DD; the item closes itself the day after.
        #[arg(long)]
        expires: Option<String>,
    },
    /// Close every open item with `--key`, or the one with `--id` — never both.
    Close {
        #[arg(long)]
        key: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long = "closed-by", default_value = "quinn")]
        closed_by: String,
    },
    /// Every open item.
    List,
}

#[derive(Subcommand)]
enum IssueArgs {
    /// Open an issue against a note, by id or vault-relative path.
    Open {
        target: String,
        /// Repeatable.
        #[arg(long = "category", value_parser = issues::CATEGORIES)]
        category: Vec<String>,
        #[arg(long, default_value = "")]
        text: String,
    },
    /// Open issues, or every issue with `--all`.
    List {
        #[arg(long)]
        all: bool,
    },
    /// Mark an issue addressed and settle it into `archive/`.
    Address {
        issue_id: String,
        #[arg(long, required = true)]
        resolution: String,
        #[arg(long)]
        commit: Option<String>,
    },
}

#[derive(Subcommand)]
enum RunsCommand {
    /// Record one step against an open run.
    Step {
        run_id: String,
        #[arg(long, required = true)]
        name: String,
        #[arg(long, required = true, value_parser = ["ok", "WARN", "FAIL"])]
        result: String,
        /// `name=int`, repeatable.
        #[arg(long, num_args = 0..)]
        counts: Vec<String>,
        #[arg(long, default_value = "")]
        message: String,
    },
    /// For each runner and each due time in the last 24h: seen | late | missing | crashed.
    ///
    /// A LATE run must not be acted on. On 2026-08-26 the scheduler fired 31 minutes late and a
    /// dead run and a late one looked identical for the first half hour.
    Status,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Rank { vault, today, runner, run_id } => {
            match cli::run(&vault, today.as_deref(), &runner, run_id.as_deref()) {
                Ok(outcome) => {
                    println!("wrote {}", outcome.output.display());
                    println!("{} {}", outcome.status, outcome.summary);
                    // Wave 3 has not ported every step `cli.run` performs. Printing the list on
                    // every run is deliberate: a missing pass that says nothing is how event
                    // discovery dies quietly at cutover.
                    if !outcome.not_ported.is_empty() {
                        eprintln!("not ported yet: {}", outcome.not_ported.join(", "));
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("knowlu-engine: {err}");
                    ExitCode::from(err.exit_code())
                }
            }
        }
        Command::Surface { vault, view, today, now, seen_at, build_sha, window } => {
            let Some(view) = knowlu_engine::surface::View::parse(&view) else { eprintln!("knowlu-engine: unknown view {view:?}"); return ExitCode::from(2); };
            let tz = knowlu_engine::cli::vault_zone(&vault);
            let today_date = match today { Some(t) => match t.parse::<jiff::civil::Date>() { Ok(d) => d, Err(_) => { eprintln!("knowlu-engine: bad --today {t:?}"); return ExitCode::from(2); } }, None => jiff::Zoned::now().with_time_zone(tz.clone()).date() };
            let now_zoned = match now { Some(n) => match n.parse::<jiff::civil::DateTime>().and_then(|d| d.to_zoned(tz.clone())) { Ok(z) => z, Err(_) => { eprintln!("knowlu-engine: bad --now {n:?}"); return ExitCode::from(2); } }, None => jiff::Zoned::now().with_time_zone(tz) };
            let mut state = match window.as_deref() {
                None => knowlu_engine::surface::build_state(&vault, view, today_date, &now_zoned, seen_at.as_deref()),
                Some(w) => match knowlu_engine::surface::build_state_preview(&vault, view, today_date, &now_zoned, seen_at.as_deref(), w) { Ok(s) => s, Err(e) => { eprintln!("knowlu-engine: bad --window: {e}"); return ExitCode::from(2); } },
            };
            if let Some(sha) = build_sha { state.topline.engine_build = Some(sha); state.revision = knowlu_engine::surface::revision_of(&state); }
            println!("{}", knowlu_engine::surface::state_json(&state));
            ExitCode::SUCCESS
        }
        // commitments command: begin
        Command::Commitments { vault, today, json, confirm, actor, via } => {
            if let Some(path) = confirm {
                let text = match std::fs::read_to_string(&path) {
                    Ok(text) => text,
                    Err(err) => {
                        eprintln!("knowlu-engine: --confirm {}: {err}", path.display());
                        return ExitCode::from(2);
                    }
                };
                let ctx = write::WriteContext::new(&actor, &via);
                return match cli::commitments_confirm(&vault, today.as_deref(), &text, &ctx) {
                    Ok(report) => {
                        println!("{}", knowlu_engine::ledger::dumps_value(&report.to_json()));
                        ExitCode::SUCCESS
                    }
                    Err(err) => {
                        eprintln!("knowlu-engine: {err}");
                        ExitCode::from(2)
                    }
                };
            }
            let report = cli::commitments_report(&vault, today.as_deref());
            if json {
                // Plan ruling Q2-b: the screen's rows in §5.2's card order.
                let mut ordered: Vec<&knowlu_engine::commitments::Proposal> = report.proposals.iter().collect();
                ordered.sort_by(|a, b| knowlu_engine::commitments::card_order(a, b));
                let value = serde_json::json!({
                    "proposals": ordered.into_iter().map(cli::proposal_json).collect::<Vec<_>>(),
                    "uncovered_courses": report.uncovered,
                    "warnings": report.warnings,
                });
                println!("{}", knowlu_engine::ledger::dumps_value(&value));
            } else {
                for p in &report.proposals {
                    println!("{}", cli::proposal_line(p));
                }
                for w in &report.warnings {
                    eprintln!("warning: {w}");
                }
            }
            ExitCode::SUCCESS
        }
        // commitments command: end
        Command::Coursework { vault, dry_run, via, run_id } => {
            match coursework::main(&vault, dry_run, &via, run_id.as_deref()) {
                0 => ExitCode::SUCCESS,
                _ => ExitCode::FAILURE,
            }
        }
        Command::CourseworkDiscover { vault, zybooks_target, vhl_target } => {
            println!(
                "{}",
                coursework::discover_json(vault.as_deref(), zybooks_target.as_deref(), vhl_target.as_deref())
            );
            ExitCode::SUCCESS
        }
        Command::Ingest { vault, via, run_id } => match ingest::run(&vault, &via, run_id.as_deref()) {
            0 => ExitCode::SUCCESS,
            _ => ExitCode::FAILURE,
        },
        Command::Judge { vault, via, run_id, runtime, model, log_dir, limit } => {
            // Always SUCCESS: `enrich::run` only ever returns 0, and this arm says so out loud
            // rather than mapping a code that cannot occur.
            let _ = enrich::run(
                &vault, &via, run_id.as_deref(), runtime.as_ref(), model.as_ref(), log_dir.as_ref(), limit,
            );
            ExitCode::SUCCESS
        }
        Command::Runs { vault, command } => match command {
            RunsCommand::Step { run_id, name, result, counts, message } => {
                let parsed = match runs::parse_counts(&counts) {
                    Ok(parsed) => parsed,
                    Err(err) => {
                        eprintln!("knowlu-engine: {err}");
                        return ExitCode::from(2);
                    }
                };
                let pairs: Vec<(&str, i64)> =
                    parsed.iter().map(|(k, v)| (k.as_str(), *v)).collect();
                runs::add_step(&vault, &run_id, &name, &result, &pairs, &message, None);
                println!("recorded step {name} on {run_id}");
                ExitCode::SUCCESS
            }
            RunsCommand::Status => match runs::expected_status(&vault, jiff::Timestamp::now()) {
                Ok(rows) => {
                    for row in rows {
                        println!("{:6} due {}  {}", row.runner, row.due, row.status);
                    }
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("knowlu-engine: {err}");
                    ExitCode::FAILURE
                }
            },
        },
        Command::Info { vault, actor, via, command } => {
            let command = match command {
                InfoArgs::Open { title, kind, body, opened_by, close_key, expires } => {
                    InfoCommand::Open { title, kind, body, opened_by, close_key, expires }
                }
                InfoArgs::Close { key, id, closed_by } => InfoCommand::Close { key, id, closed_by },
                InfoArgs::List => InfoCommand::List,
            };
            print_lines(info::cli(&vault, &actor, &via, &command))
        }
        Command::Issues { vault, actor, via, command } => {
            let command = match command {
                IssueArgs::Open { target, category, text } => {
                    IssueCommand::Open { target, categories: category, text }
                }
                IssueArgs::List { all } => IssueCommand::List { all },
                IssueArgs::Address { issue_id, resolution, commit } => {
                    IssueCommand::Address { issue_id, resolution, commit }
                }
            };
            print_lines(issues::cli(&vault, &actor, &via, &command))
        }
        Command::Write { vault, actor, via, run_id, command } => {
            let command = match command {
                WriteArgs::Set { target, pairs, judged, propose, inputs, evidence } => {
                    WriteCommand::Set { target, pairs, judged, propose, inputs, evidence }
                }
                WriteArgs::Create { rel_path, from_file } => WriteCommand::Create { rel_path, from_file },
                WriteArgs::Delete { target } => WriteCommand::Delete { target },
                WriteArgs::Move { target, new_rel } => WriteCommand::Move { target, new_rel },
                WriteArgs::AppendBody { target, line } => WriteCommand::AppendBody { target, line },
            };
            print_lines(write::cli(&vault, &actor, &via, run_id.as_deref(), &command))
        }
    }
}

/// What `engine.info.main` / `engine.issues.main` print, or the exit code they would have died
/// with: 2 for an argparse `parser.error`, 1 for an uncaught `ValueError`.
fn print_lines(result: Result<Vec<String>, info::CliError>) -> ExitCode {
    match result {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("knowlu-engine: {err}");
            ExitCode::from(err.exit_code())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The last line of `test_write.py::test_cli_set_and_create_round_trip`: a `--via` outside
    /// `VIAS` is argparse's `choices` refusal, `SystemExit(2)`. Here that is clap's, and it has
    /// to stay clap's — `write::cli` does not re-check it, exactly as `coursework` does not.
    #[test]
    fn write_refuses_a_via_outside_vias_before_touching_the_vault() {
        let err = Cli::try_parse_from([
            "knowlu-engine", "write", "--vault", ".", "--via", "carrier-pigeon", "set", "tasks/cli.md", "progress=1",
        ])
        .err()
        .expect("clap must refuse");
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
        assert_eq!(err.exit_code(), 2);
    }

    /// argparse's `nargs="+"`: `set` with no pairs is a usage error, not an empty write.
    #[test]
    fn write_set_needs_at_least_one_pair() {
        let err = Cli::try_parse_from(["knowlu-engine", "write", "set", "tasks/cli.md"]).err().expect("clap must refuse");
        assert_eq!(err.exit_code(), 2);
        assert!(Cli::try_parse_from(["knowlu-engine", "write", "set", "tasks/cli.md", "progress=1"]).is_ok());
        assert!(Cli::try_parse_from(["knowlu-engine", "write", "append-body", "tasks/cli.md", "--line", "x"]).is_ok());
    }

    /// The judge step is a runner's write like coursework's and ingest's: `--via` is validated by
    /// clap against `VIAS`, and the vocabulary does not grow for it.
    #[test]
    fn judge_takes_the_runners_via_and_refuses_anything_outside_vias() {
        assert!(Cli::try_parse_from(["knowlu-engine", "judge", "--vault", ".", "--via", "local-runner"]).is_ok());
        let err = Cli::try_parse_from(["knowlu-engine", "judge", "--via", "knowlu"]).err().expect("clap must refuse");
        assert_eq!(err.exit_code(), 2);
        assert!(Cli::try_parse_from(["knowlu-engine", "judge", "--runtime", "a.exe", "--model", "b.gguf", "--log-dir", "c", "--limit", "10"]).is_ok());
    }

    /// m3 (Task 7 fix round 1): `--limit 0` can never make progress (`0 item(s) ... 1 left for
    /// the next slot`, forever), so it is a usage error rather than a silent no-op run.
    #[test]
    fn judge_refuses_a_zero_limit() {
        let err = Cli::try_parse_from(["knowlu-engine", "judge", "--limit", "0"]).err().expect("clap must refuse");
        assert_eq!(err.exit_code(), 2);
    }
}
