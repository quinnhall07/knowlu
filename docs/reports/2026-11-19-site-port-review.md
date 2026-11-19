PASS

# Site port review: site/port-live-1119 (PR #82)

Diff: `git diff origin/main..HEAD`, one commit `c6018fe3`. Six files under `site/`, nothing on the
contract list. Scope: is the port faithful to the live `.superpowers/demo/site-dist`, and is it safe.
Nothing found that fails it.

## (a) The six files are byte-identical to site-dist

`git hash-object --no-filters` of each site-dist file equals the committed blob in `--raw`:

| file | blob |
|---|---|
| site/index.html | ed422a68 |
| site/css/hero.css | 745a83e1 |
| site/css/price.css | d4175da7 |
| site/js/dots.js | 78255f53 |
| site/js/pilot.js (new) | 67d8a3c7 |
| site/js/pilot-form.js (new) | 73507bd6 |

## (b) Nothing else differs

- All 59 tracked `site/` files were hashed raw (`--no-filters`) on three sides: working tree, index
  blob and site-dist. 58 match on all three. The one exception is `site/releases/.gitkeep`, which
  site-dist does not have.
- `git diff --no-index` from site-dist to `site/` shows only `.wrangler/cache/*` and
  `releases/{Knowlu-setup.exe, Knowlu_0.1.5_x64-setup.exe, latest.json}`, which are absent from
  `site/`. `/site/releases/*` is gitignored, and release.yml fills it at release time.
- So no file in site-dist is missing from `site/` apart from those, and `site/` has nothing extra.

## (c) Line endings and secrets

- `git ls-files --eol`: all six files are `i/lf w/lf attr/text=auto eol=lf`. Neither new file's
  blob contains a CR byte. `scripts/ci/eol-check.ps1` passes for these paths.
- No secret, token or key is in the six files. The secret pattern matched only the word "tokens"
  in `css/tokens.css` references. The Google Form ids and `entry.*` field ids are public and
  already live.

## (d) Network destinations of the new JS

**`js/pilot-form.js`**
- On submit, it sends one POST with `fetch(form.action, { method: 'POST', mode: 'no-cors' })` to
  `https://docs.google.com/forms/d/e/1FAIpQLSfXfZie7NPpQH9Y26-1HdKTxfK33ZndbwoU6FBFbWkqA9TiKA/formResponse`.
  The action is in `index.html:334`. The body is URL-encoded:
  - `entry.2058486394`: the name (trimmed);
  - `entry.1258614817`: the school email (trimmed; it passes only if it matches the regex);
  - `entry.1841072310`: once for each experience checkbox ticked, from none to eight;
  - `pageHistory=0,1`.
- `fetch`'s default credentials mode is `same-origin`, so no Google cookies are sent.
- It sends nothing if the honeypot `#pk-site` has text, or if a field fails its check.
- Without JS, the same form does a native POST to the same address.
- It stores nothing: no localStorage, no cookie.
- It also rewrites the `href` of every `[data-pf-full]` link. These are the "full application"
  links to `https://docs.google.com/forms/d/e/1FAIpQLSd7BqTSwq7x_6HlJW_NcDQK_EF9v3OE8PNyOiaIzFZN5t3MSA/viewform?usp=pp_url&entry.1521613834=The website (knowlu.com)`.
  To that `href` it appends:
  - `&entry.2017985301=<name>`;
  - `&entry.683863841=<email>`, only when the email is valid.
- Those are navigations the visitor clicks, not requests the script makes.
- It dispatches a local `kl:joined` DOM event, which `dots.js` listens for. That event never
  leaves the page.

**`js/pilot.js`**: no network and no storage. It only animates the pilot steps' line, using
scroll, resize, ResizeObserver and `document.fonts`.

**`js/dots.js`** (changed): no network calls.

**CSS** (`hero.css`, `price.css`): no `url()` or `@import` added.

**index.html**:
- Every URL the diff adds is one of the two Google Form addresses above: the header CTA, the hero
  CTA, the letter link, the form action and two full-application links.
- The external citation links (acha.org, pathify.com, insidehighered.com) were already on
  `origin/main`.
- The only scripts it adds are the two local files, `js/pilot.js` and `js/pilot-form.js`, both
  `defer`.

## (e) Nothing else in the repo depends on what changed

- **release.yml:259, 298**:
  - `pages deploy site` deploys the folder whole and `site/releases/**` is filled by the
    workflow, so neither depends on any content in the files that changed.
  - After this PR, a `v*` tag deploys exactly what is live. That is the PR's purpose, and it now
    holds.
- **engine/tests/site.rs**: checked by reading, not by running cargo, which this role may not do.
  - `the_index_makes_no_claim_the_product_cannot_back`: the ported index contains none of
    `FERPA`, `compliant`, `certified`.
  - `no_shipped_site_file_carries_a_review_aid_or_a_made_up_address`: no `REVIEW ONLY`,
    `Review copy` or `example.edu` anywhere under `site/`.
  - `every_site_font_has_its_licence_and_a_full_file`: the CSS diff adds no `@font-face`,
    `font-family` or `url(`.
  - `the_release_drop_folder_stays_in_git`: `.gitkeep` is still there.
  - The privacy-page tests are unaffected, because `privacy.html` did not change.
- **app/tests/ui_wizard.rs:269, bundle_config.rs:142, ui_settings.rs:122**: these read only
  `site/privacy.html` or `site/terms.html`, and neither file changed.
- **app/static**: nothing reads `site/index.html`, `site/js` or `site/css`. The tokens.css
  "waitlist field" comment is the app's own.
- **CSP**:
  - `site/` has no `_headers` file and no CSP, so the new `fetch` to docs.google.com is not
    blocked and needs nothing changed.
  - The app's CSP (`app/tauri.conf.json`, and `scripts/ui-fixture/serve.py`, which reads it) does
    not cover `site/`.

## For Quinn (all already live; none is caused by this PR)

1. **"Windows and Mac"**: the meta description at `index.html:9`, and the page copy, say Windows
   and Mac. Knowlu is Windows-only (CLAUDE.md). Known, and live.
2. **The privacy page does not name Google Forms.**
   - The live homepage sends a name, a school email and up to eight experience answers to Google
     Forms, and `privacy.html` does not say so.
   - Recommendation: add the pilot form to the privacy page in its own PR, as the Contract
     workflow requires.
3. **Name and email go into a URL.**
   - Once a visitor types them, the full-application links carry the name and email in the query
     string (`pilot-form.js` `prefill()`).
   - On a click, they reach Google's request logs and the browser's history.
   - Recommendation: keep the prefill, because it saves retyping, and name it on the privacy page
     in the same edit as item 2.
4. **Two comments are now out of date.**
   - `site/js/site.js:2` says "Nothing is sent from the page". `engine/tests/site.rs:37` describes
     the homepage as "early access with a mailto form".
   - Both were true of the old page. Neither breaks a test.
   - `site.js` is byte-identical to live, so changing it here would break this port's
     faithfulness.
   - Recommendation: fix the test comment in a later small change, and fix `site.js` in the site
     source, not in this port.
5. **The hidden pricing section still has the mailto waitlist form.**
   - The `hidden` `#pricing` section (`index.html:403-420`) still contains the old waitlist form,
     wired to `site.js`'s mailto.
   - It does nothing until it is unhidden, and the comment at `index.html:402` says how to bring
     it back.
   - Recommendation: when pricing returns, decide whether the waitlist goes or moves to the pilot
     form.

`pilot-form.js:2` names Quinn as the person the form emails. This does not break rule 1: it names
the founder on a public page, not a vault, machine, account or credential.
