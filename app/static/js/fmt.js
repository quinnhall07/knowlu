/* Knowlu app: the one date formatter and the course table (plan §3; the design system's fmt.js, cohesion plan B9).
   Every page calls these; none writes its own date string or its own course lookup.

   KnowluFmt: the date rule.
     due today        Tonight 11:59 PM        (the due time is 6 PM or later)    Today 4 PM    (before 6 PM)
     due tomorrow     Tomorrow 9 AM
     next few days    Thu 11:59 PM            (2 to 5 days ahead: the weekday alone is never ambiguous)
     later or past    Mon Nov 23, 9 AM        (always with the month; yesterday: Yesterday 11:59 PM)
     no time          Fri Nov 20              (Today / Tomorrow for those two days)
     a window         Wed Nov 18, 4:30 to 6:30 PM     (today: 4:30 to 6:30 PM, no date)
   A bare "weekday + number" is never produced. lead() is the lead card's and Lock in's form: Due tonight, 11:59 PM.
   Input: a Date, a number of ms, "YYYY-MM-DD" (a day) or "YYYY-MM-DDTHH:MM" (local, as the read model writes it).
   "Now" is the read model's S.now (note D5): the state the poll last painted, else the machine's clock.
     KnowluFmt.parse(x) -> {d, time} or null      KnowluFmt.now() -> Date
     KnowluFmt.clock(d) "4:30 PM"   .date(d) "Wed Nov 18"   .day(x) "Today" / "Tomorrow" / "Yesterday" / the date
     KnowluFmt.due(x)   .lead(x)   .span(a, b)   .minutes(n) "45 min", "2 h", "3 h 30 min"
     KnowluFmt.read(x) "Read 7:30 PM"   .since(x) "Not read since Sat Nov 14"   .done(x) "Done today"
     KnowluFmt.grade(g, hidden) "95.2% A", "No scores", "Hidden" (ruling 6; g is one of S.grades)

   KnowluCourses: the read model's S.courses (note §7), one entry per course note.
     KnowluCourses.all()          every entry, in the engine's order: by title (slots are given by slug)
     KnowluCourses.active(c)      true when the entry's status is absent or "active", as the engine reads it (M1)
     KnowluCourses.of(slug)       the entry whose slug is this, else null
     KnowluCourses.match(course)  of(course), else the entry whose codes hold compact(course) (N7), else null
     KnowluCourses.compact(s)     the ASCII letters and digits of s, upper-cased ("abc-101" -> "ABC101")
     KnowluCourses.slot(c)        "c1" to "c16" for an entry, "" for none (a page draws ink: neutral)
   Makes no request. */
(function () {
  "use strict";
  var WD = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  var MO = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  var STAMP = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2})(?::\d{2})?)?$/;
  var SLOT = /^c([1-9]|1[0-6])$/;

  function parse(x) {
    if (x == null || x === "") { return null; }
    if (x instanceof Date) { return { d: x, time: true }; }
    if (typeof x === "number") { return { d: new Date(x), time: true }; }
    var m = STAMP.exec(String(x));
    if (!m) { return null; }
    return { d: new Date(+m[1], +m[2] - 1, +m[3], m[4] ? +m[4] : 0, m[5] ? +m[5] : 0), time: !!m[4] };
  }
  function state() { return window.KnowluData ? window.KnowluData.state() : null; }
  function now() {
    var s = state();
    var p = s && s.now ? parse(s.now) : null;
    return p ? p.d : new Date();
  }
  function midnight(d) { return new Date(d.getFullYear(), d.getMonth(), d.getDate()); }
  /* whole calendar days from today to d (negative: past) */
  function ahead(d) { return Math.round((midnight(d) - midnight(now())) / 86400000); }
  function clock(d) {
    var hr = d.getHours(), mi = d.getMinutes(), h12 = hr % 12 || 12;
    return h12 + (mi ? ":" + (mi < 10 ? "0" : "") + mi : "") + " " + (hr < 12 ? "AM" : "PM");
  }
  function date(d) { return WD[d.getDay()] + " " + MO[d.getMonth()] + " " + d.getDate(); }
  function day(x) {
    var p = parse(x); if (!p) { return ""; }
    var n = ahead(p.d);
    return n === 0 ? "Today" : n === 1 ? "Tomorrow" : n === -1 ? "Yesterday" : date(p.d);
  }
  function due(x) {
    var p = parse(x); if (!p) { return ""; }
    var d = p.d, n = ahead(d);
    if (!p.time) { return n === 0 ? "Today" : n === 1 ? "Tomorrow" : date(d); }
    var t = clock(d);
    if (n === 0) { return (d.getHours() >= 18 ? "Tonight " : "Today ") + t; }
    if (n === 1) { return "Tomorrow " + t; }
    if (n === -1) { return "Yesterday " + t; }
    if (n >= 2 && n <= 5) { return WD[d.getDay()] + " " + t; }
    return date(d) + ", " + t;
  }
  function lead(x) {
    var p = parse(x); if (!p) { return ""; }
    var d = p.d, n = ahead(d);
    if (!p.time) { return "Due " + (n === 0 ? "today" : n === 1 ? "tomorrow" : date(d)); }
    var t = clock(d);
    if (n === 0) { return "Due " + (d.getHours() >= 18 ? "tonight" : "today") + ", " + t; }
    if (n === 1) { return "Due tomorrow, " + t; }
    if (n === -1) { return "Due yesterday, " + t; }
    if (n >= 2 && n <= 5) { return "Due " + WD[d.getDay()] + ", " + t; }
    return "Due " + date(d) + ", " + t;
  }
  function span(a, b) {
    var pa = parse(a), pb = parse(b); if (!pa || !pb) { return ""; }
    var da = pa.d, db = pb.d;
    var pre = ahead(da) === 0 ? "" : date(da) + ", ";
    if (midnight(da).getTime() !== midnight(db).getTime()) { return pre + clock(da) + " to " + date(db) + ", " + clock(db); }
    var ta = clock(da), tb = clock(db);
    if (ta.slice(-2) === tb.slice(-2)) { ta = ta.slice(0, -3); }
    return pre + ta + " to " + tb;
  }
  function minutes(n) {
    n = Math.round(n);
    if (n < 60) { return n + " min"; }
    var hr = Math.floor(n / 60), mi = n % 60;
    return hr + " h" + (mi ? " " + mi + " min" : "");
  }
  function read(x) { var p = parse(x); return p ? "Read " + (ahead(p.d) === 0 ? clock(p.d) : date(p.d) + ", " + clock(p.d)) : ""; }
  function since(x) { var p = parse(x); return p ? "Not read since " + date(p.d) : ""; }
  function done(x) { var p = parse(x); return p ? "Done " + (ahead(p.d) === 0 ? "today" : date(p.d)) : ""; }
  function grade(g, hidden) {
    if (hidden) { return "Hidden"; }
    if (!g || typeof g.pct !== "number") { return "No scores"; }
    return (Math.round(g.pct * 10) / 10).toFixed(1) + "%" + (g.letter ? " " + g.letter : "");
  }

  window.KnowluFmt = { parse: parse, now: now, clock: clock, date: date, day: day, due: due, lead: lead, span: span, minutes: minutes, read: read, since: since, done: done, grade: grade };

  function all() { var s = state(); return s && Array.isArray(s.courses) ? s.courses : []; }
  function active(c) {
    if (!c) { return false; }
    var st = String(c.status == null ? "" : c.status).trim();
    return st === "" || st === "active";
  }
  function compact(s) { return String(s == null ? "" : s).replace(/[^A-Za-z0-9]/g, "").toUpperCase(); }
  function of(slug) {
    var list = all();
    for (var i = 0; i < list.length; i++) { if (list[i].slug === slug) { return list[i]; } }
    return null;
  }
  function match(course) {
    if (course == null || course === "") { return null; }
    var hit = of(course);
    if (hit) { return hit; }
    var key = compact(course), list = all();
    if (!key) { return null; }
    for (var i = 0; i < list.length; i++) {
      var codes = Array.isArray(list[i].codes) ? list[i].codes : [];
      for (var j = 0; j < codes.length; j++) { if (compact(codes[j]) === key) { return list[i]; } }
    }
    return null;
  }
  function slot(c) { return c && SLOT.test(String(c.slot)) ? String(c.slot) : ""; }

  window.KnowluCourses = { all: all, active: active, of: of, match: match, compact: compact, slot: slot };
})();
