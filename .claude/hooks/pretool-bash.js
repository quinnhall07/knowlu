#!/usr/bin/env node
// PreToolUse hook for Bash and PowerShell (wired in .claude/settings.json).
// 1. Fixture guard: a shell command that writes under engine/tests/fixtures is denied. The eight
//    frozen references are denied for everyone; other fixture paths for every subagent except
//    contract-engineer (CLAUDE.md: the fixtures are on the contract list). Edit/Write are covered
//    by the deny rules in settings.json; this closes the shell route.
// 2. Cargo output trim: a plain cargo build/check/clippy/test/nextest command (optionally behind
//    `cd <dir> &&` or slot.sh) is rewritten to pipe through trim-cargo-output.js, which keeps
//    errors, warnings, failures and result lines, and saves the full log to a temp file.
//    Prefix the command with KNOWLU_RAW_OUTPUT=1 to skip the trim.
// Any internal error exits 0 with no output, so the hook never blocks a call by accident.
'use strict';
const path = require('path');

const FROZEN = [
  'golden-today-s1.md', 'golden-today-full.md', 'calendar-snapshot-gcal.md',
  'vault-full/state/events.md', 'zybooks-parsed-reference.json', 'vhl-parsed-reference.json',
  'run-records-reference.json', 'pyyaml-safe-dump-reference.json',
];

function out(obj) { process.stdout.write(JSON.stringify(obj)); process.exit(0); }
function deny(reason) {
  out({ hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision: 'deny', permissionDecisionReason: reason } });
}

const norm = (s) => s.replace(/\\/g, '/');
const isFixture = (s) => /tests\/fixtures(\/|$)/.test(norm(s));
const isFrozen = (s) => isFixture(s) && FROZEN.some((f) => norm(s).includes('tests/fixtures/' + f));

// Returns the paths a shell segment writes, as far as a conservative reading can tell.
function bashWriteTargets(seg) {
  const targets = [];
  const redir = seg.matchAll(/(?:^|[^0-9&])>>?\s*("[^"]+"|'[^']+'|\S+)/g);
  for (const m of redir) targets.push(m[1].replace(/^["']|["']$/g, ''));
  const words = seg.trim().split(/\s+/).map((w) => w.replace(/^["']|["']$/g, ''));
  while (words.length && /^\w+=/.test(words[0])) words.shift();
  const cmd = path.basename(words[0] || '');
  const args = words.slice(1).filter((w) => !w.startsWith('-'));
  if ((cmd === 'sed' || cmd === 'perl') && words.some((w) => /^-[a-z]*i/.test(w))) targets.push(...args);
  if (['tee', 'rm', 'truncate', 'dos2unix', 'unix2dos', 'touch', 'chmod', 'rmdir'].includes(cmd)) targets.push(...args);
  if (['cp', 'mv', 'install', 'rsync', 'ln'].includes(cmd) && args.length) targets.push(args[args.length - 1]);
  if (cmd === 'git' && ['checkout', 'restore', 'rm', 'mv', 'apply'].includes(words[1])) targets.push(...args.slice(1));
  return targets;
}

function guard(input) {
  const cmd = String((input.tool_input || {}).command || '');
  if (!isFixture(cmd)) return;
  let targets = [];
  if (input.tool_name === 'PowerShell') {
    if (/\b(Set-Content|Add-Content|Out-File|Copy-Item|Move-Item|Remove-Item|New-Item|Rename-Item|Clear-Content)\b|>/i.test(cmd)) targets = [cmd];
  } else {
    for (const seg of cmd.split(/&&|\|\||;|\||\n/)) targets.push(...bashWriteTargets(seg));
  }
  targets = targets.filter(isFixture);
  if (!targets.length) return;
  if (targets.some(isFrozen)) {
    deny('Blocked by .claude/hooks/pretool-bash.js: this command writes a frozen reference under engine/tests/fixtures. ' +
      'CLAUDE.md rule 2: never regenerate a frozen reference; if the engine disagrees with one, the engine is wrong.');
  }
  if (input.agent_type && input.agent_type !== 'contract-engineer') {
    deny('Blocked by .claude/hooks/pretool-bash.js: engine/tests/fixtures/** is on the contract list; only contract-engineer ' +
      '(or the main session) may write there. Hand this change to contract-engineer.');
  }
}

const CARGO = /^(?:bash\s+\S*slot\.sh\s+)?(?:\w+=\S*\s+)*cargo\s+(?:\+\S+\s+)?(?:build|check|clippy|test|nextest)\b/;

function trim(input) {
  if (input.tool_name !== 'Bash') return;
  const ti = input.tool_input || {};
  const cmd = String(ti.command || '');
  if (/KNOWLU_RAW_OUTPUT=1/.test(cmd)) return;
  // Leave anything a deny rule should see untouched: the rewrite changes the command's prefix.
  if (/--release\b|cargo\s+tauri\b/.test(cmd)) return;
  // The rewrite is auto-allowed, so the whole command must be inert: an optional `cd <plain path> &&`,
  // then one cargo command made only of plain words. No quotes, `$`, backticks, globs or
  // metacharacters anywhere, including the cd target and any VAR=value prefix.
  const m = /^cd\s+([\w./:\\-]+)\s*&&\s*(.*)$/s.exec(cmd.trim());
  const body = m ? m[2] : cmd.trim();
  if (!CARGO.test(body)) return;
  if (/[;|&<>`$"'*?(){}\[\]\n\r]/.test(body)) return;
  const filter = norm(path.join(__dirname, 'trim-cargo-output.js'));
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
    guard(input);
    trim(input);
  } catch (_) { /* never block on a hook bug */ }
  process.exit(0);
});
