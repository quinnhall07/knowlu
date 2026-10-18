#!/usr/bin/env node
// PreToolUse guard (wired in .claude/settings.json for Bash, PowerShell, Edit, Write, MultiEdit,
// NotebookEdit). In order:
// 1. Fixtures. The eight frozen references are never written, by anyone, by any route (rule 2;
//    settings.json also denies them by name). Every other path under engine/tests/fixtures is on
//    the contract list: a subagent other than contract-engineer is denied; the main session and
//    contract-engineer may write (Edit/Write keep the CRLF bytes; shell heredocs would not).
// 2. Outward-facing git and GitHub actions ask Quinn even in bypassPermissions mode: tag pushes,
//    force pushes in any spelling, gh release/variable/secret/ruleset changes (a v* tag ships a
//    release to every installed tester; CI_SELF_HOSTED would run public PRs on the laptop).
// 3. researcher may run read-only git and gh only (it also reads untrusted web pages).
// 4. Cargo output trim: an inert cargo build/check/clippy/test/nextest command (optionally behind
//    `cd <plain path> &&` or slot.sh) is rewritten to pipe through trim-cargo-output.js.
//    Prefix KNOWLU_RAW_OUTPUT=1 to skip.
// An internal error exits 0 with no output, so a hook bug never blocks a call.
'use strict';
const path = require('path');

const FROZEN = [
  'golden-today-s1.md', 'golden-today-full.md', 'calendar-snapshot-gcal.md',
  'vault-full/state/events.md', 'zybooks-parsed-reference.json', 'vhl-parsed-reference.json',
  'run-records-reference.json', 'pyyaml-safe-dump-reference.json',
];
const FROZEN_NAMES = FROZEN.map((f) => f.split('/').pop());

function out(obj) { process.stdout.write(JSON.stringify(obj)); process.exit(0); }
function decide(permissionDecision, reason) {
  out({ hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision, permissionDecisionReason: reason } });
}
const deny = (reason) => decide('deny', 'Blocked by .claude/hooks/pretool-bash.js: ' + reason);

// Windows paths are case-insensitive and tolerate `//` and `/./`, so compare a normalized form.
const norm = (s) => String(s).replace(/\\/g, '/').replace(/\/(\.\/)+/g, '/').replace(/\/{2,}/g, '/').toLowerCase();
const isFixture = (s) => /tests\/fixtures(\/|$)/.test(norm(s));
const isFrozenPath = (s) => isFixture(s) && FROZEN.some((f) => norm(s).endsWith('tests/fixtures/' + f));
// A subagent carries agent_type and agent_id; the main session carries neither.
const subagentOf = (input) => (input.agent_type || input.agent_id ? String(input.agent_type || 'unknown') : '');

const READ_ONLY = /^\s*(git\s+(diff|log|show|status|blame)|cat|head|tail|grep|rg|diff|wc|ls|sha256sum|md5sum|od|xxd)\b[^<>]*$/;
// Read-only only if every piece is: no ;, &&, || or newlines, and each |-stage is a reader.
const isReadOnly = (cmd) => !/;|&&|\|\||\n|>/.test(cmd) && cmd.split('|').every((s) => READ_ONLY.test(s));
// One writer registry for both shells, so the lists cannot drift apart.
const PS_WRITERS = 'Set-Content|Add-Content|Out-File|Copy-Item|Move-Item|Remove-Item|New-Item|Rename-Item|Clear-Content|Set-ItemProperty|WriteAllText|WriteAllBytes|WriteAllLines';
const CHDIR = 'cd|chdir|pushd|Set-Location|Push-Location|sl';
// Anything that can write without naming the file as a plain argument.
const OPAQUE_WRITER = new RegExp(`\\b(${CHDIR}|python3?|py|node|deno|perl|ruby|sh|bash|pwsh|powershell|patch|awk|xargs)\\b|>|` +
  `\\b(sed|tee|cp|mv|rm|truncate|install|rsync|dos2unix|unix2dos)\\b|git\\s+(apply|checkout|restore|am)\\b|\\b(${PS_WRITERS})\\b`, 'i');
const PS_WRITE = new RegExp(`\\b(${PS_WRITERS}|${CHDIR})\\b|>`, 'i');

// The paths a shell segment writes, as far as a conservative reading can tell.
function bashWriteTargets(seg) {
  const targets = [];
  for (const m of seg.matchAll(/(?:^|[^0-9&])>>?\s*("[^"]+"|'[^']+'|\S+)/g)) targets.push(m[1].replace(/^["']|["']$/g, ''));
  const words = seg.trim().split(/\s+/).map((w) => w.replace(/^["']|["']$/g, ''));
  while (words.length && /^\w+=/.test(words[0])) words.shift();
  const cmd = path.basename(words[0] || '');
  const args = words.slice(1).filter((w) => !w.startsWith('-'));
  if ((cmd === 'sed' || cmd === 'perl') && words.some((w) => /^-[a-z]*i|^--in-place/.test(w))) targets.push(...args);
  if (['tee', 'rm', 'truncate', 'dos2unix', 'unix2dos', 'touch', 'chmod', 'rmdir', 'patch'].includes(cmd)) targets.push(...args);
  if (['cp', 'mv', 'install', 'rsync', 'ln'].includes(cmd) && args.length) targets.push(args[args.length - 1]);
  for (const w of words) { const o = /^--output=(.+)/.exec(w); if (o) targets.push(o[1]); }
  if (cmd === 'git' && ['checkout', 'restore', 'rm', 'mv', 'apply', 'am'].includes(words[1])) targets.push(...args.slice(1));
  return targets;
}

function guardFixtures(input) {
  const ti = input.tool_input || {};
  const agent = subagentOf(input);
  const contractOnly = 'engine/tests/fixtures/** is on the contract list; only contract-engineer (or the main session) ' +
    'may write there. Hand this change to contract-engineer.';
  const frozen = 'this writes a frozen reference. CLAUDE.md rule 2: never regenerate a frozen reference; if the engine ' +
    'disagrees with one, the engine is wrong.';

  if (['Edit', 'Write', 'MultiEdit', 'NotebookEdit'].includes(input.tool_name)) {
    const p = ti.file_path || ti.notebook_path || '';
    if (!isFixture(p)) return;
    if (isFrozenPath(p)) deny(frozen);
    if (agent && agent !== 'contract-engineer') deny(contractOnly);
    return;
  }

  const cmd = String(ti.command || '');
  const n = norm(cmd);
  if (FROZEN_NAMES.some((f) => n.includes(f)) && OPAQUE_WRITER.test(cmd) && !isReadOnly(cmd)) {
    deny('this command names a frozen reference and can write files. CLAUDE.md rule 2: never regenerate a frozen ' +
      'reference. To read one, use Read, or cat/diff/git diff on their own.');
  }
  if (!isFixture(cmd)) return;
  let targets = [];
  if (input.tool_name === 'PowerShell') {
    if (PS_WRITE.test(cmd)) targets = [cmd];
  } else {
    for (const seg of cmd.split(/&&|\|\||;|\||\n/)) targets.push(...bashWriteTargets(seg));
  }
  targets = targets.filter(isFixture);
  if (!targets.length) return;
  if (targets.some(isFrozenPath)) deny(frozen);
  if (agent && agent !== 'contract-engineer') deny(contractOnly);
}

const OUTWARD = [
  [/\bgit\b[^\n]*\bpush\b[^\n]*(--tags|--follow-tags|--mirror|refs\/tags\/|(^|[\s:+])v\d)/i, 'pushes a tag (a v* tag ships a release to every installed tester)'],
  [/\bgit\b[^\n]*\bpush\b[^\n]*(\s-f\b|\s-[a-z]*f[a-z]*\b|--force|\s\+\S)/i, 'force-pushes'],
  [/\bgit\b[^\n]*\bpush\b[^\n]*(--delete|\s:\S)/i, 'deletes a remote ref'],
  [/\bgh\s+(release|secret|variable|ruleset)\s+(create|delete|edit|set|upload)\b/i, 'changes a GitHub release, secret, variable or ruleset'],
  [/\bgh\s+repo\s+(edit|delete|rename|archive)\b/i, 'changes the GitHub repository'],
  [/\bgh\s+api\b[^\n]*((-X|--method)[\s=]*(?!GET\b)\w|\s-[fF]\b|--field|--raw-field|--input)/i, 'writes through the GitHub API'],
  [/\bgh\s+workflow\s+(run|enable|disable)\b/i, 'starts or changes a GitHub workflow'],
];

function guardOutward(input) {
  if (input.tool_name !== 'Bash' && input.tool_name !== 'PowerShell') return;
  const cmd = String((input.tool_input || {}).command || '');
  for (const [re, what] of OUTWARD) {
    if (re.test(cmd)) decide('ask', `.claude/hooks/pretool-bash.js: this command ${what}. Releases, tags and force pushes need Quinn's go.`);
  }
}

const RESEARCHER_OK = /^\s*(git\s+(log|show|diff|blame|status|rev-parse|merge-base|ls-files|ls-tree|cat-file|for-each-ref|describe|shortlog)\b|gh\s+(pr|issue|run|release)\s+(view|list|diff|checks)\b|gh\s+repo\s+view\b)[^;&|<>`$]*$/;

function guardResearcher(input) {
  if (subagentOf(input) !== 'researcher' || input.tool_name !== 'Bash') return;
  const cmd = String((input.tool_input || {}).command || '');
  // git options that write files or run programs, even on a read-only subcommand
  const writesOrRuns = /\s(--output|-o\b|--ext-diff|--textconv|--exec|--upload-pack|--receive-pack|-c\s|--config-env)/.test(cmd);
  if (!RESEARCHER_OK.test(cmd) || writesOrRuns) {
    deny('researcher reads untrusted web pages, so its shell is limited to one read-only git or gh command ' +
      '(git log/show/diff/blame/status..., gh pr|issue|run view/list). No pipes, redirects or chaining.');
  }
}

const CARGO = /^(?:bash\s+\S*slot\.sh\s+)?(?:\w+=\S*\s+)*cargo\s+(?:\+\S+\s+)?(?:build|check|clippy|test|nextest)\b/;

function trim(input) {
  if (input.tool_name !== 'Bash') return;
  const ti = input.tool_input || {};
  const cmd = String(ti.command || '');
  if (/KNOWLU_RAW_OUTPUT=1/.test(cmd)) return;
  // Leave anything a deny rule should see untouched: the rewrite changes the command's prefix.
  if (/--release\b|\s-r\b|--profile\b|cargo\s+tauri\b/.test(cmd)) return;
  // The rewrite is auto-allowed, so the whole command must be inert: an optional `cd <plain path> &&`,
  // then one cargo command made only of plain words.
  const m = /^cd\s+([\w./:\\-]+)\s*&&\s*(.*)$/s.exec(cmd.trim());
  const body = m ? m[2] : cmd.trim();
  if (!CARGO.test(body)) return;
  if (/[;|&<>`$"'*?(){}\[\]\n\r]/.test(body)) return;
  const filter = path.join(__dirname, 'trim-cargo-output.js').replace(/\\/g, '/');
  const wrapped = `{ ${cmd.trim()}\n} 2>&1 | node "${filter}"; exit \${PIPESTATUS[0]}`;
  out({ hookSpecificOutput: {
    hookEventName: 'PreToolUse', permissionDecision: 'allow',
    permissionDecisionReason: 'cargo output trimmed by .claude/hooks/trim-cargo-output.js',
    updatedInput: Object.assign({}, ti, { command: wrapped }),
  } });
}

let raw = '';
process.stdin.on('data', (d) => { raw += d; });
process.stdin.on('end', () => {
  try {
    const input = JSON.parse(raw);
    guardFixtures(input);
    guardOutward(input);
    guardResearcher(input);
    trim(input);
  } catch (_) { /* never block on a hook bug */ }
  process.exit(0);
});
