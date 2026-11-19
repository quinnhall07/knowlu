"""Build the fixture harness's base envelopes from the engine (v0.4.0 plan T5c, P8, R2-2, R2-3).

Nothing here is written by hand into engine output. The script

1. copies three files of `engine/tests/fixtures/vault-full/config/` (read only: `events.yaml`,
   `planning.yaml`, `runners.yaml`) into a vault in a temporary folder outside the repository, so no
   fixture task, approval, course slug or class time comes along. `ingest.yaml` (whose `course_map`
   could carry fixture codes) and `week_template.yaml` (a week of class times) stay behind; the
   script writes an invented week template instead;
2. adds invented course notes (`ABC 101`, ...), invented class times and invented tasks for them;
3. runs `target/debug/knowlu-engine.exe surface --vault <tmp> --view today|all` with the surface
   oracle's pins (`--today 2026-08-28 --now 2026-08-28T09:00 --build-sha pinned`);
4. wraps each answer as the app's `state` command does: the envelope `{ok, error, state}` with the
   seven topline keys the app adds (`sync`, `backup`, `startup_missed`, `last_slot`, `scheduler`,
   `console_build`, `seen_at`; app/src/commands.rs `build_state_value` and `attach_scheduler`);
5. writes `envelopes/base-<set>-{today,all}.json`.

Sets: `courses` (the base), and the variants `overload`, `unreadable` and `digest-only`, each made by
changing the vault copy; `first-run`, `reconsent-owed`, `signed-out` and `engine-down` change only
what the app adds (the envelope's `first_run` block; or, in `base-<set>-answers.json`, the
`privacy_status` answer, a `grades_status` that is signed out, a `state` that did not answer).

Running it twice gives byte-identical envelopes: the engine is deterministic under the pins, the
added keys are constants, and the output is checked for the temporary path, for every course slug
the fixture vault's tasks name, and for any class block that is not one of the invented times.

    python scripts/ui-fixture/make_base_courses.py            # builds the engine first (-j 2)
    python scripts/ui-fixture/make_base_courses.py --no-build # uses the engine already built
"""
import argparse
import json
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent.parent
FIXTURE = REPO / "engine" / "tests" / "fixtures" / "vault-full"
ENGINE = REPO / "target" / "debug" / "knowlu-engine.exe"
OUT = HERE / "envelopes"
PINS = ["--today", "2026-08-28", "--now", "2026-08-28T09:00", "--build-sha", "pinned"]
CONFIG_COPIED = ["events.yaml", "planning.yaml", "runners.yaml"]

# Invented class times for the invented courses (ABC 101 Mon/Wed/Fri, DEF 210 Tue/Thu, JKL 120 Mon/Wed).
WEEK = {
    "mon": [("10:10", "11:00"), ("15:10", "16:00")],
    "tue": [("14:20", "15:35")],
    "wed": [("10:10", "11:00"), ("15:10", "16:00")],
    "thu": [("14:20", "15:35")],
    "fri": [("10:10", "11:00")],
    "sat": [],
    "sun": [],
}

# The seven topline keys the app adds to every `state` answer, as constants (a quiet, healthy app).
TOPLINE = {
    "console_build": None,
    "seen_at": "2026-08-27T21:00:00Z",
    "sync": {"ok": True, "at": "2026-08-28T13:00:00Z", "lines": [], "last_error": None, "skipped": None},
    "backup": {"last_ok": "2026-08-28", "behind_days": 0, "last_error": None, "target_reachable": True},
    "startup_missed": False,
    "last_slot": {
        "started": "2026-08-28T07:00:00-05:00", "ended": "2026-08-28T07:01:12-05:00",
        "steps": [["sync", 0], ["coursework", 0], ["ingest", 0], ["grades", 0], ["judge", 0], ["rank", 0]],
        "ok": True, "engine_ok": True, "late": False, "reason": None, "attempt": 1,
    },
    "scheduler": {"mode": "app", "paused": False, "device_ok": True},
}

FIRST_RUN = {"running": True, "current": "coursework", "steps": [["sync", 0]]}
PRIVACY_OWED = {"ok": True, "error": None, "current": "2026-10-20", "recorded": "2026-09-09", "needs_accept": True}
GRADES_SIGNED_OUT = {"ok": True, "available": True, "reason": None, "connected": True, "signed_out": True,
                     "host": "lms.example.edu", "fetched_at": "2026-08-27T18:00", "hidden": False}
ENGINE_DOWN = {"ok": False, "error": "the engine did not answer", "state": None}
# The sets that are the base envelopes with other answers staged beside them.
ANSWERED = {
    "reconsent-owed": {"privacy_status": PRIVACY_OWED},
    "signed-out": {"grades_status": GRADES_SIGNED_OUT},
    "engine-down": {"state": ENGINE_DOWN},
}

# Invented courses: one is dropped, so the sidebar lists three of the four.
COURSES = [
    ("abc-101", "ABC 101", "Introductory Studies", None, "ABC 101"),
    ("def-210", "DEF 210", "Applied Methods", "active", "DEF 210"),
    ("ghi-150", "GHI 150", "Survey of Ideas", "dropped", "GHI 150"),
    ("jkl-120", "JKL 120", "Writing Workshop", None, "JKL 120"),
]

# Invented tasks: (file stem, id, title, course, due, effort, importance, status, progress)
TASKS = [
    ("abc-101-problem-set-2", "task_a1b2c3d401", "ABC 101 Problem set 2", "abc-101", "2026-08-27T23:59", 1.5, 3, "active", 0),
    ("abc-101-reading-quiz", "task_a1b2c3d402", "ABC 101 Reading quiz", "abc-101", "2026-08-28T23:59", 0.5, 2, "active", 0),
    ("def-210-lab-write-up", "task_a1b2c3d403", "DEF 210 Lab write-up", "def-210", "2026-08-29T17:00", 2.0, 3, "active", 0),
    ("jkl-120-essay-draft", "task_a1b2c3d404", "JKL 120 Essay draft", "jkl-120", "2026-09-02T23:59", 3.0, 4, "active", 40),
    ("def-210-midterm-review", "task_a1b2c3d405", "DEF 210 Midterm review", "def-210", "2026-09-08T09:00", 4.0, 5, "active", 0),
    ("renew-library-card", "task_a1b2c3d406", "Renew library card", None, "2026-08-31", 0.5, 1, "active", 0),
    ("abc-101-problem-set-1", "task_a1b2c3d407", "ABC 101 Problem set 1", "abc-101", "2026-08-21T23:59", 1.5, 3, "done", 100),
]


def yaml_text(s):
    return json.dumps(s, ensure_ascii=False)


def task_note(tid, title, course, due, effort, importance, status, progress):
    return "\n".join([
        "---",
        "title: " + yaml_text(title),
        "course: " + (course if course else "null"),
        "domain: school",
        "due: " + due,
        "effort_hours: " + str(effort),
        "effort_confidence: low",
        "effort_source: inferred",
        "importance: " + str(importance),
        "status: " + status,
        "progress: " + str(progress),
        "created_by: fixture",
        "id: " + tid,
        "---",
        "",
        "Invented for the fixture harness.",
        "",
    ])


def course_note(n, slug, title, name, status, code):
    lines = ["---", "type: course", "slug: " + slug, "title: " + yaml_text(title), "name: " + yaml_text(name), "code: " + yaml_text(code)]
    if status:
        lines.append("status: " + status)
    lines += ["id: crs_f1x7ure0%02d" % n, "---", "", "Invented for the fixture harness.", ""]
    return "\n".join(lines)


def week_template():
    lines = ['day_start: "08:00"', 'day_end: "18:00"', "min_block_minutes: 45", "classes:"]
    for day, blocks in WEEK.items():
        if not blocks:
            lines.append("  %s: []" % day)
            continue
        lines.append("  %s:" % day)
        for start, end in blocks:
            lines.append('    - ["%s", "%s"]' % (start, end))
    return "\n".join(lines) + "\n"


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(text.encode("utf-8"))


def base_vault(root):
    """The base vault: three of the fixture's config files, an invented week, invented courses and tasks."""
    (root / "config").mkdir(parents=True)
    for name in CONFIG_COPIED:
        shutil.copyfile(FIXTURE / "config" / name, root / "config" / name)
    write(root / "config" / "week_template.yaml", week_template())
    for folder in ["tasks", "approvals", "archive", "courses", "info", "issues", "commitments", "state"]:
        (root / folder).mkdir(exist_ok=True)
    for n, (slug, title, name, status, code) in enumerate(COURSES, start=1):
        write(root / "courses" / (slug + ".md"), course_note(n, slug, title, name, status, code))
    for stem, tid, title, course, due, effort, imp, status, progress in TASKS:
        write(root / "tasks" / (stem + ".md"), task_note(tid, title, course, due, effort, imp, status, progress))


def overload(root):
    """Eight more tasks due today and tomorrow, three hours each: far past the 4-hour budget."""
    for i in range(8):
        due = "2026-08-28T23:59" if i % 2 == 0 else "2026-08-29T23:59"
        course = COURSES[i % 2][0]
        write(root / "tasks" / ("extra-%d.md" % i), task_note("task_0ver10ad%02d" % i, "%s Extra exercise %d" % (COURSES[i % 2][1], i + 1), course, due, 3.0, 3, "active", 0))


def unreadable(root):
    """One task note whose frontmatter does not parse."""
    write(root / "tasks" / "broken-note.md", "---\ntitle: [this list never closes\ncourse: abc-101\n---\n\nInvented.\n")


def digest_only(root):
    """Waiting holds only an events digest, two invented events."""
    write(root / "approvals" / "events-digest-2026-08-28.md", "\n".join([
        "---", "type: approval", "kind: events-digest", "title: " + yaml_text("Events - 2026-08-28"), "status: pending",
        "proposed_at: 2026-08-28", "first_proposed_at: 2026-08-28", "expires: 2026-09-01", "snooze_until: null", "created_by: events",
        "events:",
        "  - uid: " + yaml_text("fixture:event-1"), "    summary: " + yaml_text("Study skills workshop"),
        "    start: 2026-09-01T16:00", "    end: 2026-09-01T17:00", "    location: " + yaml_text("Room 101"),
        "  - uid: " + yaml_text("fixture:event-2"), "    summary: " + yaml_text("Club fair"),
        "    start: 2026-09-03T11:00", "    end: 2026-09-03T14:00", "    location: " + yaml_text("Quad"),
        "id: appr_f1x7ure001", "---", "",
        "**2 events worth a look.** Tick the ones you want on your calendar, then Approve.", "",
    ]))


def surface(vault, view):
    out = subprocess.run([str(ENGINE), "surface", "--vault", str(vault), "--view", view] + PINS, capture_output=True)
    if out.returncode != 0:
        sys.exit("surface --view %s failed: %s" % (view, out.stderr.decode("utf-8", "replace")))
    return json.loads(out.stdout.decode("utf-8"))


def envelope(state):
    top = state.setdefault("topline", {})
    for k in ["console_build", "seen_at", "sync", "backup", "startup_missed", "last_slot", "scheduler"]:
        top[k] = TOPLINE[k]
    return {"ok": True, "error": None, "state": state}


def dump(path, value):
    path.write_bytes((json.dumps(value, ensure_ascii=False, indent=1) + "\n").encode("utf-8"))


def fixture_slugs():
    slugs = set()
    for p in sorted((FIXTURE / "tasks").glob("*.md")):
        m = re.search(r"^course:\s*([A-Za-z0-9-]+)\s*$", p.read_text(encoding="utf-8"), re.M)
        if m and m.group(1) != "null":
            slugs.add(m.group(1))
    return slugs


def class_blocks(value):
    """Every (start, end) of a block of kind "class" anywhere in an envelope."""
    found = []
    if isinstance(value, dict):
        if value.get("kind") == "class" and "start" in value and "end" in value:
            found.append((value["start"], value["end"]))
        for v in value.values():
            found += class_blocks(v)
    elif isinstance(value, list):
        for v in value:
            found += class_blocks(v)
    return found


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--no-build", action="store_true", help="use the engine already in target/debug")
    args = ap.parse_args()
    if not args.no_build:
        subprocess.run(["cargo", "build", "-p", "knowlu-engine", "-j", "2"], cwd=REPO, check=True)
    if not ENGINE.is_file():
        sys.exit("no engine at %s: build it with cargo build -p knowlu-engine -j 2" % ENGINE)
    OUT.mkdir(exist_ok=True)
    banned = fixture_slugs()
    written = []
    with tempfile.TemporaryDirectory(prefix="knowlu-ui-fixture-") as tmp:
        for name, change in [("courses", None), ("overload", overload), ("unreadable", unreadable), ("digest-only", digest_only)]:
            vault = pathlib.Path(tmp) / name
            base_vault(vault)
            if change:
                change(vault)
            for view in ["today", "all"]:
                env = envelope(surface(vault, view))
                text = json.dumps(env, ensure_ascii=False)
                for needle in [tmp, tmp.replace("\\", "/"), tmp.replace("\\", "\\\\")]:
                    if needle in text:
                        sys.exit("the engine's answer names the temporary vault; the envelope would not be stable")
                path = OUT / ("base-%s-%s.json" % (name, view))
                dump(path, env)
                written.append(path)
                if name == "courses":
                    for other in ANSWERED:
                        dump(OUT / ("base-%s-%s.json" % (other, view)), env)
                        written.append(OUT / ("base-%s-%s.json" % (other, view)))
    for view in ["today", "all"]:
        path = OUT / ("base-first-run-%s.json" % view)
        dump(path, {"ok": True, "error": None, "state": None, "first_run": FIRST_RUN})
        written.append(path)
    for other, staged in ANSWERED.items():
        answers = OUT / ("base-%s-answers.json" % other)
        dump(answers, staged)
        written.append(answers)
    for path in written:
        text = path.read_text(encoding="utf-8")
        for slug in sorted(banned):
            if re.search(r"(?<![A-Za-z0-9-])" + re.escape(slug) + r"(?![A-Za-z0-9-])", text):
                sys.exit("%s carries the fixture vault's course slug %s" % (path.name, slug))
    invented = {pair for blocks in WEEK.values() for pair in blocks}
    for path in written:
        for pair in class_blocks(json.loads(path.read_text(encoding="utf-8"))):
            if pair not in invented:
                sys.exit("%s carries a class block %s-%s that is not one of the invented times" % (path.name, pair[0], pair[1]))
    for path in written:
        print(path.relative_to(REPO).as_posix())


if __name__ == "__main__":
    main()
