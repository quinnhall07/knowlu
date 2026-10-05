//! The setup runner on the real window (email-forwarding plan T11.3, H4). The one place the runner meets
//! Tauri: [`WindowHost`] is [`Host`] over the `mail` window, [`start_setup`] builds a run's [`Env`] (the
//! saved session's client, the system clock, the one-window hold) and starts it on [`SETUPS`], and
//! [`on_start`] is what the app's `setup` calls at launch (H4): the start-up sweep of `mail-setup` and of
//! deletes that failed before, then **an open backfill resumes** on its kept profile, hidden.
//!
//! A closed backfill batch starts a slot at once (spec §4.8 step 1): the hook runs `scheduler::run_slot` on
//! its own thread, which never runs two slots (a slot already in progress is left alone).
use super::host::Host;
use super::recover::{Mailbox, Place, SystemClock, WindowMailbox};
use super::setup::{Env, Kind, Plan, Shared, SETUPS};
use super::{close, come_back, open_hidden, open_visible, page_title, popup_open, step_aside, window_signed_in, MailWindow, Refusal, WINDOW};
use crate::mail_client::MailClient;
use crate::mail_session::MailState;
use crate::mail_steps::Provider;
use crate::state::ConsoleState;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

/// [`Host`] over the open `mail` window.
pub struct WindowHost {
    app: AppHandle,
    data_dir: PathBuf,
    provider: Mutex<Provider>,
}

impl WindowHost {
    pub fn new(app: AppHandle, data_dir: PathBuf) -> WindowHost {
        WindowHost { app, data_dir, provider: Mutex::new(Provider::Guided) }
    }

    fn provider(&self) -> Provider {
        *self.provider.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn mailbox(&self) -> WindowMailbox<'_> {
        WindowMailbox { app: &self.app, provider: self.provider() }
    }
}

impl Mailbox for WindowHost {
    fn script(&self, path: &str, args: &Value) -> Result<Value, Refusal> {
        self.mailbox().script(path, args)
    }

    fn open(&self, place: Place) -> Result<(), Refusal> {
        self.mailbox().open(place)
    }
}

/// The address a Gmail page's title names ([`page_title`], never the native window's): `<folder> - <address> - Gmail` (a Workspace mailbox ends with
/// its organisation's name instead). Lower case; read for the hint and kept nowhere.
fn address_in_title(title: &str) -> Option<String> {
    let parts: Vec<&str> = title.split(" - ").collect();
    if parts.len() < 3 {
        return None;
    }
    parts[1..parts.len() - 1].iter().find(|p| p.contains('@')).map(|p| p.trim().to_ascii_lowercase())
}

impl Host for WindowHost {
    fn use_provider(&self, provider: Provider) {
        *self.provider.lock().unwrap_or_else(|p| p.into_inner()) = provider;
    }

    fn open_window(&self, profile: &Path, visible: bool) -> Result<(), String> {
        if visible {
            open_visible(&self.app, &self.data_dir, profile, self.provider())
        } else {
            open_hidden(&self.app, &self.data_dir, profile, self.provider())
        }
    }

    fn window_open(&self) -> bool {
        self.app.get_webview_window(WINDOW).is_some()
    }

    fn signed_in(&self) -> bool {
        window_signed_in(&self.app, self.provider())
    }

    fn title_address(&self) -> Option<String> {
        address_in_title(&page_title(&self.app)?)
    }

    fn popup_open(&self) -> bool {
        popup_open(&self.app)
    }

    fn step_aside(&self) -> Result<(), String> {
        step_aside(&self.app)
    }

    fn come_back(&self) -> Result<(), String> {
        come_back(&self.app)
    }

    fn close_window(&self) {
        close(&self.app)
    }
}

/// The saved sign-in's client for this vault's account; a named reason when there is none.
pub fn client_for(vault: &Path) -> Result<MailClient, &'static str> {
    let cfg = crate::account::cloud_config(vault).map_err(|_| "no account")?;
    MailClient::for_session(&cfg.api_base, &cfg.anon_key, &cfg.session_credential_target).map_err(|e| e.name())
}

fn real_env(app: &AppHandle, data_dir: &Path, client: MailClient, guard: MailWindow) -> Env {
    let slot_app = app.clone();
    Env {
        host: Box::new(WindowHost::new(app.clone(), data_dir.to_path_buf())),
        client,
        clock: Box::new(SystemClock),
        // A closed backfill batch starts a slot now (§4.8 step 1): `judge` pulls it without waiting.
        on_closed: Box::new(move || {
            let app = slot_app.clone();
            std::thread::spawn(move || {
                let _ = crate::scheduler::run_slot(&app, false);
            });
        }),
        guard: Some(guard),
    }
}

/// Starts a setup for `kind` on the console's profile. `Err` is a fixed sentence: no account, the mail
/// window is busy, `mail.json` unreadable, or a setup is already running.
pub fn start_setup(app: &AppHandle, kind: Kind) -> Result<Arc<Shared>, &'static str> {
    let cs = app.try_state::<ConsoleState>().ok_or("no profile is open")?;
    let client = client_for(&cs.vault)?;
    let keep_session = MailState::load(&cs.data_dir).map_err(|_| "mail.json is unreadable")?.keep_session;
    let guard = MailWindow::try_take().ok_or("a mail window is already open")?;
    let plan = Plan { kind, data_dir: cs.data_dir.clone(), keep_session, setup_day: jiff::Zoned::now().date() };
    let env = real_env(app, &cs.data_dir, client, guard);
    SETUPS.start(plan, env).map_err(|_| "a mail setup is already running")
}

/// H4, at launch: sweeps a leftover `mail-setup` and retries deletes that failed, then resumes every open
/// backfill whose profile is kept. Never blocks the app's start: it runs on its own thread.
/// The mail window is taken on the calling thread, before the sweep's thread starts, and held across the
/// sweep and the resume (spec §5.4), so a setup started at launch is refused rather than opening WebView2
/// on a folder being deleted.
pub fn on_start(app: &AppHandle) {
    let Some(cs) = app.try_state::<ConsoleState>() else { return };
    let (vault, data_dir) = (cs.vault.clone(), cs.data_dir.clone());
    let app = app.clone();
    let window = MailWindow::try_take();
    std::thread::spawn(move || {
        let Some(guard) = sweep_at_launch(&data_dir, window) else { return };
        if !MailState::load(&data_dir).is_ok_and(|s| s.keep_session) {
            return;
        }
        let ids = super::backfill::open_backfills(&data_dir);
        if ids.is_empty() {
            return;
        }
        let Ok(client) = client_for(&vault) else { return };
        let env = real_env(&app, &data_dir, client, guard);
        let _ = SETUPS.start_resume(data_dir, ids, env);
    });
}

/// The start-up sweep (`mail_session::start_up_sweep`), run only while `window` holds the mail window:
/// `None` (another holder has it) sweeps nothing and hands nothing back; a held window is handed back
/// for the resume. Its outcome is the sweep's own to record (`pending_deletes`).
pub fn sweep_at_launch(data_dir: &Path, window: Option<MailWindow>) -> Option<MailWindow> {
    let window = window?;
    let _ = crate::mail_session::start_up_sweep(data_dir);
    Some(window)
}
