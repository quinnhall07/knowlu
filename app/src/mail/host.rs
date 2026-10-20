//! The mail window as the setup runner drives it (email-forwarding plan T11.3). A trait, so the runner
//! is tested against a fake window and the app's one real implementation ([`super::live::WindowHost`])
//! is the only code here that touches Tauri. A [`Mailbox`] already runs a compiled-in script by name and
//! goes to a place; the rest is the window itself: open it on a profile, ask whether the student closed
//! it, whether the page looks signed in, whether Google's popup is up, and step it aside or bring it back.
//!
//! Nothing here keeps or returns a page's text beyond [`Host::title_address`], which the runner sends to
//! the service once (the mailbox's `mailbox_hint`, spec §4.1 step 3) and drops.
use super::recover::Mailbox;
use crate::mail_steps::Provider;
use std::path::Path;

pub trait Host: Mailbox + Send {
    /// The provider of the mailbox the next scripts run for (a resume handles mailboxes of both).
    fn use_provider(&self, provider: Provider);
    /// Opens the `mail` window on `profile`, visible (to sign in) or hidden (a saved sign-in's run).
    fn open_window(&self, profile: &Path, visible: bool) -> Result<(), String>;
    /// Whether the window is still there: `false` once the student has closed it.
    fn window_open(&self) -> bool;
    /// X2's title signal: the page is on a mail host and its title reads as a signed-in mailbox.
    fn signed_in(&self) -> bool;
    /// Gmail: the address the window's title names, which is the mailbox's own (spec §4.3 step 1).
    fn title_address(&self) -> Option<String>;
    /// Whether Google's "verify it's you" popup is open.
    fn popup_open(&self) -> bool;
    /// The window steps aside: hidden and out of the taskbar (D5).
    fn step_aside(&self) -> Result<(), String>;
    /// The site needs the student: the window comes back, centred and focused (D5).
    fn come_back(&self) -> Result<(), String>;
    /// Closes the popup and the window. The profile folder is `mail_session`'s to delete.
    fn close_window(&self);
}
