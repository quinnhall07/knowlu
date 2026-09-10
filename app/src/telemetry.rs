//! Telemetry (a) and (b), derived on the device (Knowlu C1, Task 15). Spec §6, decided under D5.
//!
//! **What may leave, and nothing else.** (a) is `state/events-ui/`'s ledger, which `uievents::record`
//! already refuses free text into. (b) is derived here from the journal: a human, at the console,
//! setting a field an agent had set. Its values travel only for fields whose vocabulary is closed or
//! numeric; a **course** correction keeps its row and loses its two names, and a **title** correction
//! is not a row at all.
use std::path::{Path, PathBuf};

/// Judged fields whose correction may carry its values: numbers and closed vocabularies only.
///
/// **Fix round 1 (C1, ruling R-C1-39):** three of these five — `domain`, `effort_confidence` and
/// `status` — are free-text inputs in the console today (`commands.rs`'s `QUOTED` list, and
/// `console.js` puts them in plain text boxes) with no vocabulary check anywhere in the write path.
/// This constant still names them, matching the brief and the already-shipped `handler.ts` list, but
/// `is_wire_value` below is the belt that keeps a sentence typed into one of those three from ever
/// reaching `ours`/`theirs` — the list alone is not the guarantee.
pub const VALUED_FIELDS: [&str; 5] = ["effort_hours", "importance", "domain", "effort_confidence", "status"];
/// Judged fields recorded as "it changed" and nothing more. A course name is somebody's timetable.
pub const FLAGGED_FIELDS: [&str; 1] = ["course"];

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct EventRow {
    pub ts: String,
    pub session: String,
    pub view: String,
    pub action: String,
    pub object_id: Option<String>,
    pub object_kind: Option<String>,
    pub ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CorrectionRow {
    pub ts: String,
    pub item_id: String,
    pub field: String,
    pub ours: Option<String>,
    pub theirs: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Batch {
    pub events: Vec<EventRow>,
    pub corrections: Vec<CorrectionRow>,
}

/// Every `.jsonl` day file under `dir`, in filename order, each line parsed. A line that does not
/// parse is skipped rather than fatal: a half-written last line is what a power cut leaves, and one
/// lost event is not worth losing a batch over.
fn read_ledger(dir: &Path) -> Vec<serde_json::Map<String, serde_json::Value>> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<PathBuf> = rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "jsonl").unwrap_or(false))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        for line in text.lines() {
            if let Ok(serde_json::Value::Object(m)) = serde_json::from_str(line) {
                out.push(m);
            }
        }
    }
    out
}

fn s(m: &serde_json::Map<String, serde_json::Value>, k: &str) -> Option<String> {
    m.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

/// A scalar as the one short string the wire carries. `2.0` stays `2.0`, not `2`: the journal's own
/// spelling is what an eval example must compare against later.
fn scalar(v: Option<&serde_json::Value>) -> Option<String> {
    match v {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::Bool(b)) => Some(b.to_string()),
        _ => None,
    }
}

/// `tasks/a.md` → `task`. A token, never a path: the folder is the kind and the filename is a title.
fn kind_of(path: &str) -> String {
    let top = path.split('/').next().unwrap_or("");
    top.strip_suffix('s').unwrap_or(top).to_string()
}

/// `engine/src/uievents.rs`'s `is_token` character class, mirrored rather than imported: that
/// function is private to the engine crate, and this is the same property this crate needs on the
/// device side of the wire — alphanumeric, `_-:`, at most 64 characters, never empty.
fn is_token(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':')
}

/// **Fix round 1 (C1, ruling R-C1-39).** The one guard between a `VALUED_FIELDS` correction and free
/// text on the wire: a numeric literal (so `2.0`, `-1`, `0.5` all pass, whatever `scalar` produced
/// from the journal's `new`), or a short closed-vocabulary-shaped token. A field like `domain` typed
/// as a sentence in the console fails both and is treated exactly like `course` — the row survives,
/// the value does not.
fn is_wire_value(s: &str) -> bool {
    s.parse::<f64>().is_ok() || is_token(s)
}

/// (a). `since` is **exclusive** — the watermark is the last `ts` that was accepted, so re-running a
/// slot after a network failure sends the same rows again and the server's unique constraint absorbs
/// them; re-running after a success sends nothing.
pub fn read_events(vault: &Path, since: Option<&str>) -> Vec<EventRow> {
    read_ledger(&vault.join("state").join("events-ui"))
        .into_iter()
        .filter_map(|m| {
            let ts = s(&m, "ts")?;
            if let Some(w) = since {
                if ts.as_str() <= w { return None; }
            }
            Some(EventRow {
                ts,
                session: s(&m, "session")?,
                view: s(&m, "view")?,
                action: s(&m, "action")?,
                object_id: s(&m, "object_id"),
                object_kind: s(&m, "object_kind"),
                ms: m.get("ms").and_then(|v| v.as_i64()),
            })
        })
        .collect()
}

/// (b). Spec §6: "every human override of a judged field — journal records with `via: dashboard` on a
/// field the agent set". Walks the journal in order, remembering who last set each `(id, field)`; a
/// dashboard write over an `agent:` write is a correction, and nothing else is.
///
/// **Fix round 1 (M5):** only `op == "set"` records are read at all — neither to learn "who set this
/// last" nor to emit a row. Today every record carrying a `field` is a `set`, so this changes no
/// existing fixture's result; it keeps a future `op` (an unset, a revert) from being silently read as
/// "the agent set this".
pub fn read_corrections(vault: &Path, since: Option<&str>) -> Vec<CorrectionRow> {
    let mut last: std::collections::BTreeMap<(String, String), (String, Option<String>)> = std::collections::BTreeMap::new();
    let mut out = Vec::new();
    for m in read_ledger(&vault.join("state").join("journal")) {
        let (Some(ts), Some(id), Some(field), Some(actor)) = (s(&m, "ts"), s(&m, "id"), s(&m, "field"), s(&m, "actor")) else { continue };
        if s(&m, "op").as_deref() != Some("set") { continue; }
        let key = (id.clone(), field.clone());
        let value = scalar(m.get("new"));
        let via = s(&m, "via").unwrap_or_default();
        let human = via == "dashboard" && !actor.starts_with("agent:");
        if human {
            if let Some((prior_actor, prior_value)) = last.get(&key) {
                if prior_actor.starts_with("agent:") {
                    let valued = VALUED_FIELDS.contains(&field.as_str());
                    let flagged = FLAGGED_FIELDS.contains(&field.as_str());
                    let fresh = since.map(|w| ts.as_str() > w).unwrap_or(true);
                    if (valued || flagged) && fresh {
                        // Fix round 1 (C1): `is_wire_value` bounds what actually reaches `ours`/
                        // `theirs` — a sentence typed into a free-text VALUED field drops out here,
                        // the same "it changed" treatment `course` (a FLAGGED field) already gets.
                        let keep = |v: &Option<String>| v.as_deref().filter(|s| is_wire_value(s)).map(str::to_string);
                        out.push(CorrectionRow {
                            ts: ts.clone(),
                            item_id: id.clone(),
                            field: field.clone(),
                            ours: if valued { keep(prior_value) } else { None },
                            theirs: if valued { keep(&value) } else { None },
                            kind: kind_of(&s(&m, "path").unwrap_or_default()),
                        });
                    }
                }
            }
        }
        last.insert(key, (actor, value));
    }
    out
}

/// The last `ts` this profile has sent. A file beside `seen.txt`, for the reason the entitlement
/// cache is one: `state::Settings` cannot grow a field without breaking every existing settings file.
pub fn watermark_path(data_dir: &Path) -> PathBuf { data_dir.join("telemetry-sent.txt") }

pub fn load_watermark(data_dir: &Path) -> Option<String> {
    std::fs::read_to_string(watermark_path(data_dir)).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn save_watermark(data_dir: &Path, ts: &str) -> Result<(), String> {
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    std::fs::write(watermark_path(data_dir), ts).map_err(|e| e.to_string())
}

/// A failed `POST /telemetry`, split the way `send` needs to tell the outcomes apart (fix round 1,
/// I2): a status the server actually answered with, or a transport failure before one ever arrived.
/// Neither variant carries a response body or the token — a server's own message is not this crate's
/// business to relay, and nothing about the token belongs in an error at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostError {
    Status(u16),
    Transport(String),
}

impl std::fmt::Display for PostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostError::Status(status) => write!(f, "telemetry: refused ({status})"),
            PostError::Transport(e) => write!(f, "telemetry: {e}"),
        }
    }
}

pub fn post_batch_at(api_base: &str, token: &str, batch: &Batch) -> Result<(usize, usize), PostError> {
    crate::account::check_api_base(api_base).map_err(PostError::Transport)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    // Compact, not `send_json`: `app/Cargo.toml` does not enable ureq's `json` feature, exactly the
    // reason `account::post_json` gives for the same choice.
    let body = serde_json::to_string(batch).map_err(|e| PostError::Transport(e.to_string()))?;
    let mut res = agent
        .post(&format!("{}/telemetry", api_base.trim_end_matches('/')))
        .header("authorization", &format!("Bearer {token}"))
        .header("content-type", "application/json")
        .send(body)
        .map_err(|e| PostError::Transport(e.to_string()))?;
    let status = res.status().as_u16();
    let text = res.body_mut().with_config().limit(1 << 16).read_to_string().map_err(|e| PostError::Transport(e.to_string()))?;
    if !(200..300).contains(&status) { return Err(PostError::Status(status)); }
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    Ok((
        v.get("events").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
        v.get("corrections").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
    ))
}

/// **How far the watermark may advance.** A stream that hit the cap contributes only its own last
/// `ts` and the answer is the **earliest** of the contributions: taking the maximum across both would
/// step past a truncated stream's tail, so a vault with 600 pending events and one late correction
/// would advance past events 501-600 and never send them. With neither stream capped there is nothing
/// to step past and the answer is the latest of the two. An empty batch has no watermark.
///
/// Its own function because it is the one piece of arithmetic here that is wrong silently — the
/// symptom is events that were never sent and nothing anywhere saying so.
pub fn watermark(last_event: Option<&str>, last_correction: Option<&str>, capped: bool) -> String {
    let ends = [last_event, last_correction].into_iter().flatten();
    if capped { ends.min() } else { ends.max() }.unwrap_or_default().to_string()
}

/// **Fix round 1 (M2).** A capped page must never end on a `ts` it shares with a row that got cut:
/// `watermark` takes the kept page's last `ts` as the new floor, and `since` is exclusive, so a
/// watermark landing mid-tie would skip that tie's other rows on every later read, forever — the
/// same silent-loss shape the cap itself exists to avoid. If the row that would be the first one
/// dropped (`rows[cap]`) shares its `ts` with the last row the naive truncation would keep, the whole
/// trailing run sharing that `ts` is dropped from the kept side too, deferring the tie whole to the
/// next read.
///
/// **Fix round 2.** The tie-cut above has its own edge, when the tie is not a boundary pair but the
/// *entire* page: if every one of `rows[0..cap]` shares `rows[cap]`'s `ts` (an unrealistic burst —
/// 500+ rows in one millisecond is a corrupt or hand-edited ledger, not a real cap boundary),
/// stopping at `keep == 1` would still set the watermark to that shared `ts` and silently strand the
/// other ~499 rows carrying it forever — M2's own failure, just at a wider tie. Cutting all the way
/// to an *empty* page would fix that but trade it for a worse one: an empty batch on every slot,
/// forever, for a vault whose ledger keeps producing this — `send` would defer this same page and
/// never make progress. So this case keeps the **whole original page** instead (all `cap` rows,
/// ignoring the tie): it delivers the most of the page in one slot, and the watermark then advances
/// past the tie exactly as it does today — only the rows past `cap` that share the tie are lost, not
/// the whole page. Once the vault stops producing runs this long, `rows.len() <= cap` and the whole
/// page sends normally with no loss at all.
///
/// Public for the same reason `watermark` is: the one piece of arithmetic here that is wrong
/// silently, and a test needs to reach it directly rather than through `send`'s network call.
pub fn truncate_on_ts_boundary<T>(rows: &mut Vec<T>, cap: usize, ts_of: impl Fn(&T) -> &str) {
    if rows.len() <= cap { return; }
    let boundary_ts = ts_of(&rows[cap]).to_string();
    let mut keep = cap;
    while keep > 1 && ts_of(&rows[keep - 1]) == boundary_ts {
        keep -= 1;
    }
    // Fix round 2: the tie reaches all the way to index 0 — the entire page is one tie, with no
    // non-tied prefix to cut at. Keep the whole page rather than the one row `keep == 1` would leave.
    if keep == 1 && ts_of(&rows[0]) == boundary_ts {
        keep = cap;
    }
    rows.truncate(keep);
}

/// A completed `send`: nothing pending, or a batch the server actually took a position on.
///
/// **Fix round 1 (I2).** A refusal still advances the watermark past the batch — see `send` — because
/// a status in [`is_permanently_refused`]'s set means the batch is malformed and will never become
/// well-formed by being sent again unchanged. `Sent` and `Refused` are both success outcomes from
/// `send`'s own point of view: the watermark moved either way. A transport failure, a rate limit, an
/// auth hiccup or a 5xx is not this type at all — those stay `Err(String)` on `send`, keep the batch,
/// and are retried at the next slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOutcome {
    Sent(usize, usize),
    Refused(u16),
}

/// The three statuses a malformed batch can wear that will never resolve themselves by retrying:
/// bad request, payload too large, unprocessable. Everything else — an auth failure, a rate limit,
/// any 5xx — is a reason to try again later, not a reason to give up on the batch.
fn is_permanently_refused(status: u16) -> bool {
    matches!(status, 400 | 413 | 422)
}

/// One send: read since the watermark, post, advance the watermark on any outcome the server took a
/// position on. **At most 500 of each** — the server's cap — so a vault with a long history catches
/// up over several slots instead of being refused forever.
pub fn send(vault: &Path, data_dir: &Path, vault_io: &std::sync::Mutex<()>) -> Result<SendOutcome, String> {
    let cfg = crate::account::cloud_config(vault)?;
    let since = load_watermark(data_dir);
    // Fix round 1 (M4): read-only, but still vault I/O, so it takes `vault_io` the way the `backup`
    // step does (F11) — a console-initiated sync rewriting the working tree mid-read must not be able
    // to make `read_ledger` silently skip a day file and advance the watermark past it. Released
    // before the network call: a POST may take up to 30s and must never hold a lock a sync is
    // waiting on, the same reason `vault_io` is never held across the engine child processes either.
    let (mut events, mut corrections) = {
        let _io = vault_io.lock().unwrap_or_else(|e| e.into_inner());
        (read_events(vault, since.as_deref()), read_corrections(vault, since.as_deref()))
    };
    // **The watermark can only advance as far as the SLOWER stream got** — `watermark` above is that
    // rule, and the reason it is a rule. This is what makes "catches up over several slots" true.
    let capped = events.len() > 500 || corrections.len() > 500;
    truncate_on_ts_boundary(&mut events, 500, |e| e.ts.as_str());
    truncate_on_ts_boundary(&mut corrections, 500, |c| c.ts.as_str());
    if events.is_empty() && corrections.is_empty() { return Ok(SendOutcome::Sent(0, 0)); }
    let last_event = events.last().map(|e| e.ts.clone());
    let last_correction = corrections.last().map(|c| c.ts.clone());
    let high = watermark(last_event.as_deref(), last_correction.as_deref(), capped);
    if high.is_empty() { return Ok(SendOutcome::Sent(0, 0)); }
    let auth = crate::account::auth_base(&cfg.api_base)?;
    let token = crate::account::valid_access_token_at(&auth, &cfg.anon_key, &cfg.session_credential_target, jiff::Timestamp::now().as_second())?;
    match post_batch_at(&cfg.api_base, &token, &Batch { events, corrections }) {
        Ok((e, c)) => {
            save_watermark(data_dir, &high)?;
            Ok(SendOutcome::Sent(e, c))
        }
        // Fix round 1 (I2): the server has taken a position — this batch is malformed and will never
        // become well-formed — so it is consumed exactly as a successful send would be, rather than
        // rebuilt and re-sent at every slot forever.
        Err(PostError::Status(status)) if is_permanently_refused(status) => {
            save_watermark(data_dir, &high)?;
            Ok(SendOutcome::Refused(status))
        }
        Err(e) => Err(e.to_string()),
    }
}
