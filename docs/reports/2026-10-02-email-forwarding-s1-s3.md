# Email forwarding T0.S1-S3: the Tauri 2 host checks, live

Date: 2026-10-02. Result: **S1a, S1b, S1c, S2 and S3 all passed live.** Source of record: the ledger row
"T0.S1-S3 live" in the email-forwarding progress ledger. Plan section: T0.S1-S3.

## Setup

- A throwaway Tauri 2 app in the scratchpad, on the app's own pins: tauri 2.11.5, wry 0.55.1,
  webview2-com 0.38.2.
- A self-driving harness: the controller drove the spike over a loopback control port. There was no
  debugging port, no capability grant and no OS input (no synthetic keyboard or mouse).
- Quinn was at the machine for every sign-in and phone tap.
- Two real mailbox sessions (Outlook and Gmail), each in its own `data_directory` profile.

## S1a. A script's result comes back without IPC: PASS

- Channel 1, WebView2 `ExecuteScript` completion through `with_webview`: Outlook's message list, 7,512 B,
  in 10 ms. Gmail's list, 6,508 B.
- Channel 2, the title channel (fallback): the same Outlook list in 4 chunks, 22 ms. **One title caps at
  4,096 chars**, so the channel is chunked.
- `__TAURI__` is undefined in both pages. `__TAURI_INTERNALS__` is present (see Quinn's ruling below), but
  every invoke was denied.
- Channel 3 (sentinel navigation) was not needed.

## S1b. Google's "verify it's you" popup: PASS

- Gmail's "Add a forwarding address" opened the popup through `on_new_window` -> `Create`, in the same
  profile, with `window_features`.
- The first request is `about:blank`, so the popup filter must allow `about` first.
- `window.opener` was non-null after 2.0 s on the Google accounts page. Quinn tapped the phone. The popup
  closed itself via the mail-settings page (`WindowCloseRequested` -> `destroy()`), and the opener showed
  "confirmation link sent".

## S1c. The origin check: PASS

- A script queued before a navigation ran on Outlook at 0 ms (before commit) and was refused at 20-400 ms
  after a cross-origin navigation.
- So the host-side check is not enough alone: **scripts carry an in-page `location.host` guard** as well.

## S2. Hidden or off-screen: PASS (both modes)

- Hidden (`visible(false)` + `skip_taskbar(true)`), 10 min: 21 of 21 reads (median 5 ms), 2 of 2 harmless
  clicks took effect and were restored.
- Off-screen, 3 min: 10 of 10 reads, 2 of 2 clicks.
- The forwarding toggle reads by `input[role=switch]` `.checked`.
- The main window stayed responsive: at most 2 ms, at 10 Hz, throughout.
- **Sender form.** Outlook: the sender element's `title` attribute is always the address (the visible text
  was a display name 3 of 8 times, a bare address 5 of 8). Gmail: the `span[email]` attribute is always the
  address (the display name showed 12 of 14 times). The fingerprint reads the address from those attributes.
- **Received time.** Visible text is `h:mm am/pm` for today and `Mon d` for older mail (Gmail). The time
  element's `title` attribute carries the full date and time on both, so the fingerprint reads the title.
- **Profile deletion.** After close, the profile folder deleted 1.3 s later (Outlook) and 0.8 s (Gmail),
  each on the second attempt of a 250 ms retry.
- Two `data_directory` profiles opened one after the other in one process: OK.

## S3. The protocol-handler prompt: PASS

Outlook and Gmail each called `registerProtocolHandler` once. An initialization script swallowed the call,
and Quinn saw no prompt. The host also cancels `LaunchingExternalUriScheme`.

## What T9.2 and T10.x build on

- **Result channel:** `ExecuteScript` completion via `with_webview`. The title channel is the fallback,
  chunked below 4,096 chars per title.
- **Scripts:** wrapped in try/catch and returning JSON; each carries an in-page `location.host` guard in
  addition to the host-side origin check.
- **Hide mode:** `visible(false)` + `skip_taskbar(true)` (off-screen also passed).
- **Popups:** `on_new_window` -> `Create` with `window_features`, same profile, `about:blank` allowed first;
  `WindowCloseRequested` -> `destroy()`.
- **Protocol prompt:** an initialization script suppresses `registerProtocolHandler`, plus the
  `LaunchingExternalUriScheme` cancel.
- **`forget`:** profile delete retries every 250 ms (succeeded 0.8-1.3 s after close).
- **Fingerprint:** sender from the title attribute (Outlook) or `span[email]` attribute (Gmail); received
  time from the time element's title attribute.

## The origin-list finding

Outlook now lives at `outlook.cloud.microsoft`; `outlook.office.com` redirects there. **The origin list
must include `outlook.cloud.microsoft`.** Without it, the runner would refuse Outlook's own pages. The plan's
T9.2 now says so.

## Quinn's IPC ruling (2026-10-02)

Tauri injects `__TAURI_INTERNALS__` into every webview, and it cannot be removed. Accepted, on the
condition that **no capability covers any mail window and every invoke is denied** (both observed in S1a).
T9.2 adds a test that fails if any capability file (`app/capabilities/*.json`) names a mail window label or a
remote URL.

## Cleanup

- The spike app and both WebView2 profile folders were deleted.
- The raw message-list dumps were deleted.
- Quinn removed the test forwarding addresses and the S0 filter from the mailboxes.
