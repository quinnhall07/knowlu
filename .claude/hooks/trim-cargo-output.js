#!/usr/bin/env node
// Filter for cargo output, piped in by pretool-bash.js. Short output passes through unchanged.
// Long output keeps: error/warning diagnostics (with their location and note lines), panics,
// the failures section, every `test result:` line, slot.sh lines and the last lines; the full
// log is saved to a temp file whose path is printed, so nothing needs a re-run to be read.
'use strict';
const fs = require('fs');
const os = require('os');
const path = require('path');

const PASS_THROUGH = 80;    // lines; shorter output is printed whole
const BLOCK_CAP = 25;       // lines kept per diagnostic block
const TAIL = 6;

let raw = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (d) => { raw += d; });
process.stdin.on('end', () => {
  const lines = raw.split('\n');
  if (lines.length <= PASS_THROUGH) { process.stdout.write(raw); return; }

  let logPath = '';
  try {
    logPath = path.join(os.tmpdir(), `knowlu-cargo-${Date.now()}-${process.pid}.log`);
    fs.writeFileSync(logPath, raw);
  } catch (_) { logPath = ''; }

  const keep = new Array(lines.length).fill(false);
  let passed = 0, failed = 0, ignored = 0, warnings = 0;
  for (let i = 0; i < lines.length; i++) {
    const l = lines[i];
    const r = /^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored/.exec(l);
    if (r) { passed += +r[1]; failed += +r[2]; ignored += +r[3]; keep[i] = true; continue; }
    if (/^(error|warning)(\[\w+\])?:/.test(l)) {
      if (l.startsWith('warning')) warnings++;
      // a diagnostic block runs until the next blank line
      for (let j = i, n = 0; j < lines.length && lines[j].trim() !== '' && n < BLOCK_CAP; j++, n++) keep[j] = true;
      continue;
    }
    if (/panicked at|^---- .* ----$|^failures:$|FAILED|^\s+Running |\[slot\.sh\]|could not compile|error: test failed/.test(l)) {
      keep[i] = true;
    }
    if (/^---- .* ----$/.test(l)) {
      for (let j = i + 1, n = 0; j < lines.length && !/^---- .* ----$|^failures:$/.test(lines[j]) && n < 40; j++, n++) keep[j] = true;
    }
  }
  for (let i = Math.max(0, lines.length - TAIL); i < lines.length; i++) keep[i] = true;

  // Running lines are only worth keeping next to a failure; drop them when everything passed.
  if (failed === 0) {
    for (let i = 0; i < lines.length; i++) if (/^\s+Running /.test(lines[i])) keep[i] = false;
  }

  const kept = [];
  let gap = 0;
  for (let i = 0; i < lines.length; i++) {
    if (keep[i]) {
      if (gap) kept.push(`  [... ${gap} lines]`);
      gap = 0;
      kept.push(lines[i]);
    } else gap++;
  }
  if (gap) kept.push(`  [... ${gap} lines]`);

  const head = `[cargo output trimmed: ${kept.length} of ${lines.length} lines kept` +
    (passed + failed + ignored ? `; totals ${passed} passed, ${failed} failed, ${ignored} ignored` : '') +
    `; ${warnings} warning line(s)` +
    (logPath ? `; full log: ${logPath.replace(/\\/g, '/')}` : '') +
    '; prefix KNOWLU_RAW_OUTPUT=1 for untrimmed output]';
  process.stdout.write(head + '\n' + kept.join('\n') + '\n');
});
