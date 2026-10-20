# Email forwarding: T0 report (gates), 2026-10-01

Status: **S0's DMARC gate passed.** S0's real-forward captures are pending. S1-S3 are taken from the
controller's spike, which covered part of the plan's list (see "Gaps"). S4, S5 and S6 are not started.
Rule 1: no account id, address, key or token appears here.

## S0. The receiver and the DMARC gate

**Setup (as built)**
- Region: SES `us-east-2`. The new AWS sign-up fixes the region, and SES receiving is supported there
  (`inbound-smtp.us-east-2.amazonaws.com`).
- Domain `in.knowlu.com` is verified by DKIM CNAMEs in Cloudflare.
- Bucket: public access blocked, AES256 at rest, one-day expiry on `inbound/` and `work/`.
- SNS topic: publish allowed to SES only.
- Active rule set `knowlu-mail`, one rule `inbound`: an S3 action with `TopicArn` set and nothing else.
  Spam and virus scan on. Nothing is rejected, bounced or stopped at the door.

**Smoke test.** A message from a founder Gmail was stored, with every verdict PASS.

**The DMARC gate.** A deliberately failing sender: a throwaway subdomain publishing `p=reject`, sent
through SES with DKIM signing off and MAIL FROM `amazonses.com`.

| Check | Result |
|---|---|
| SPF | PASS, unaligned |
| DKIM | GRAY |
| DMARC | FAIL, policy reject |
| Receipt action | S3 (not rejected) |
| Stored object | 3104 B, AES256, one-day expiry present |

Pass condition met: the DMARC-failing message is in S3 with its verdicts recorded, and nothing was
bounced or dropped. The door accepts mail that fails DMARC, so the later rules judge it, not SES.

**Cleanup.** The throwaway sender identity was deleted.

**Gmail forwarding confirmation (E3).** Completed by a cookie-less POST from the stored confirmation
mail. The response read "Confirmation Success!".

**Pending (S0 captures).** M365 forward, Gmail filter forward, intra-school message, and
forward-as-attachment from each provider. Still to record: the SNS notification fields, ARC instances
and sealing domains, AMS `h=` lists, envelope-sender shape and SES's SPF verdict on it, Microsoft's
`ForwardingLoop` field, which `Authentication-Results` each provider stamps, the intra-school
alignment, and whether a forward-as-attachment keeps delivery headers. If the intra-school message is
not aligned, Quinn hears the proportion before T2f fixes the `aligned` rule (spec section 4.6 step 6).
The gate that blocks cloud tasks (S0 passes) is cleared on the DMARC result; the header shapes still
gate the tasks that depend on them.

## S1-S3. Tauri 2 host checks (from the controller's spike)

Spike pins: tauri 2.11.5, wry 0.55.1, webview2-com 0.38.2. It builds with only the accepted `.rsrc`
warning. Test pages came from a server on `127.0.0.1:0`, which Tauri treats as a remote origin. The spike
lives in the scratchpad and never enters the repo. Its own check names (Q1-Q5) differ from the plan's
S-numbers; the mapping is given per section.

### Hidden window (plan S2): works. Use `hide()`.
- API: `data_directory(fresh dir)` with an external URL. After a "signed in" beacon, Rust calls
  `hide()`, `minimize()` or `set_position(-20000,-20000)`.
- 60 s each: hidden, minimised and off-screen all ran at the visible rate. `setInterval(100 ms)` at
  10.00 Hz, rAF at 60 Hz, DOM mutations at 4 Hz, 60 of 60 fetch beacons arrived. Rust's eval returned in
  10 ms or less.
- The page never knew it was hidden: `visibilityState` stayed `visible`. Tauri's hide touches only the
  outer window.
- Throttling comes only from setting WebView2 `IsVisible(false)` yourself (`with_webview`): timers fall
  to 0.98 Hz, rAF to 0, 35 of 60 beacons, first eval 963 ms. The flags
  `--disable-background-timer-throttling --disable-renderer-backgrounding
  --disable-backgrounding-occluded-windows` restore 8.4 Hz and 58 of 60 beacons.
- `TrySuspend` freezes the page (0 Hz, eval answers with frozen state); `Resume` restores 10 Hz.
- Caveat: a hidden page still renders at 60 fps, so it costs as much CPU and GPU as a visible one.
  Chromium's 5-minute intensive throttling was not tested; it applies only to pages that know they are
  hidden, which `hide()` never causes.

### Popups (plan S1b): works, one caveat.
- API: `on_new_window` returning `Create { window }`, with the window built at `about:blank` and
  `.window_features(features)`. That copies the opener's WebView2 environment, so the popup shares the
  mail profile (the cookie was present). `Allow` makes WebView2's own popup, which has no Tauri IPC.
- A `window.open` from eval with no click was not blocked. In both modes the popup saw its opener,
  messages went both ways, and the opener saw `popup.closed`. A popup opened visibly while the mail
  window was hidden.
- Caveat: when a `Create` popup calls `window.close()`, the Tauri window stays as an empty frame. Fix,
  verified once: a `WindowCloseRequested` handler on the popup that calls `destroy()` from another
  thread.
- `show()`, `unminimize()` and `set_focus()` bring the hidden mail window back.
- Not tested: real user interaction (synthetic input is off limits). Popups reported
  `is_focused=false` despite `.focused(true)`.

### Results back to Rust (plan S1a): works.
- API: `eval_with_callback` returns the result as JSON text. It needs no permission and works on a
  remote page.
- Caveats: a Promise or async function returns `"{}"`, so async results need IPC or a second read. A
  thrown error returns `"null"`, the same as `undefined`, so scripts must catch their own errors.
- IPC from the remote page works with an app permission manifest: `build.rs` lists the commands, and
  one capability grants windows `mail*` on remote URL `http://127.0.0.1:*/*` only `allow-mail-result`.
  `mail_result` ran; `forbidden_cmd` was rejected.
- The controller's findings text is cut off after that point (see "Gaps").

### Further results from the spike log (not in the findings text)
- Navigations to `mailto:` and `ms-outlook:` schemes (`on_navigation` deny or allow, anchor click,
  `window.open`): the page stayed alive on its URL each time (relates to plan S3; the Outlook prompt
  itself was not tested).
- Two `data_directory` profiles were open at once in one process, and the main window stayed
  responsive (counter kept advancing). Closing one left the other alive.
- Profile folders: the first delete right after `destroy` failed (file in use); a retry succeeded after
  about 300 ms (2 tries) for both folders. This supports a retry in `forget`.

### Gaps against the plan's S1-S3 list
Not covered by the spike text or log: the 4 KB result from Outlook's real message list and `__TAURI__`
absence on Outlook; Google's real "verify it's you" popup with a phone tap; the origin check (S1c); the
ten-minute run on Outlook's settings page with a scripted click; each provider's sender and time form
in the message list; and S3's `registerProtocolHandler` suppression. Per P4, no task that touches the
window or a step script starts until these pass. The result channel (`eval_with_callback`, with IPC as
the fallback), the hide mode (`hide()`) and the popup route (`on_new_window` with `Create`) that T9.2
and T10.2a build on are recorded above.

### Cleanup
The spike's profile folders were used on test pages only, not real mailbox sessions. The spike app's
deletion is to be recorded here when the remaining checks finish.
