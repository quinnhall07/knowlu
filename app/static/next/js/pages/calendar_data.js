/* Knowlu app: the Calendar's data (v0.4.0 note §4 "Calendar", plan T11). Pure: it reads the read model and the
   your_week answer and returns plain lists; it draws nothing and sends nothing. The grid (calendar.js) calls it.

   KnowluCalData.iso(d)                 "YYYY-MM-DD" for a Date (local)
   KnowluCalData.day(s)                 the Date (local midnight) of "YYYY-MM-DD" or "YYYY-MM-DDTHH:MM", else null
   KnowluCalData.add(d, n)              d moved n days         .monday(d)  the Monday of d's week (the week starts Monday)
   KnowluCalData.mins("HH:MM")          minutes since midnight
   KnowluCalData.expand(week, from, n)  your_week.week.commitments[] expanded per day for n days from the Date from: an
                                        object keyed by date, each a list of {key, id, title, kind, level, course, where,
                                        s, e, start, end, date, slot}, by start. A commitment meets on each of its meets'
                                        days inside its own from and until (inclusive); its class slot comes from
                                        KnowluCourses.match (slug, else compact(course) in an entry's codes, N7), and
                                        with no match it is neutral ("").
   KnowluCalData.unconfirmed(state, today)  today's S.the_day.blocks of kind "busy" that no commitment covers (the
                                        engine draws a confirmed commitment's own block too, labelled with its title);
                                        A6: the_day is today only, so other days have none
   KnowluCalData.accepted(state)        S.coming_up entries with accepted true (M3), each with its date and time
   KnowluCalData.dayLoad(state, date)   the hours a day needs: S.ahead.buckets' hours inside the fourteen buckets;
                                        beyond them, the summed hours of the open rows due that day; null before today or
                                        with nothing due
   KnowluCalData.dueOn(state, date)     the open rows of S.list due that day, in the list's order
   KnowluCalData.lanes(items)           things at the same time share the column: sets li (its lane) and ln (lanes in its
                                        cluster) on each item, returns them by start */
(function () {
  "use strict";
  var DAY_KEYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
  var STAMP = /^(\d{4})-(\d{2})-(\d{2})/;

  function pad(n) { return (n < 10 ? "0" : "") + n; }
  function iso(d) { return d.getFullYear() + "-" + pad(d.getMonth() + 1) + "-" + pad(d.getDate()); }
  function day(s) {
    var m = STAMP.exec(String(s == null ? "" : s));
    return m ? new Date(+m[1], +m[2] - 1, +m[3]) : null;
  }
  function add(d, n) { return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n); }
  /* Monday is day 0 of the engine's week (DAY_KEYS) */
  function keyIndex(d) { return (d.getDay() + 6) % 7; }
  function monday(d) { return add(d, -keyIndex(d)); }
  function mins(hm) {
    var m = /^(\d{1,2}):(\d{2})/.exec(String(hm == null ? "" : hm));
    return m ? +m[1] * 60 + +m[2] : null;
  }
  function list(x) { return Array.isArray(x) ? x : []; }

  function slotOf(course) {
    var entry = course == null || course === "" ? null : KnowluCourses.match(course);
    return entry ? KnowluCourses.slot(entry) : "";
  }

  function expand(week, from, n) {
    var out = {};
    var dates = [];
    for (var i = 0; i < n; i++) {
      var d = add(from, i);
      dates.push(d);
      out[iso(d)] = [];
    }
    list(week && week.commitments).forEach(function (cm) {
      var first = cm["from"] ? String(cm["from"]).slice(0, 10) : "";
      var last = cm["until"] ? String(cm["until"]).slice(0, 10) : "";
      var slot = slotOf(cm.course);
      list(cm.meets).forEach(function (m, mi) {
        var s = mins(m.start), e = mins(m.end);
        if (s == null || e == null || e <= s) { return; }
        var days = list(m.days);
        dates.forEach(function (d) {
          var date = iso(d);
          if ((first && date < first) || (last && date > last)) { return; }
          if (days.indexOf(DAY_KEYS[keyIndex(d)]) < 0) { return; }
          out[date].push({
            key: String(cm.id || cm.title) + "|" + mi + "|" + date, id: cm.id || null, title: cm.title || "", kind: cm.kind || "",
            level: cm.level || "hard", course: cm.course || null, where: cm.where || "", s: s, e: e, start: m.start, end: m.end,
            date: date, slot: slot
          });
        });
      });
    });
    Object.keys(out).forEach(function (k) { out[k].sort(function (a, b) { return a.s - b.s || a.e - b.e; }); });
    return out;
  }

  /* a busy block the engine drew for a confirmed span: same title, inside the commitment's span (it clamps to the
     window, so the drawn block can be shorter than the meeting) */
  function covered(block, occs) {
    var s = mins(block.start), e = mins(block.end);
    return occs.some(function (o) { return o.title === block.label && o.s <= s && e <= o.e; });
  }
  function unconfirmed(state, today) {
    var occs = list(today);
    var blocks = list(state && state.the_day && state.the_day.blocks);
    return blocks.filter(function (b) { return b.kind === "busy" && !covered(b, occs); }).map(function (b, i) {
      return { key: "busy|" + i + "|" + b.start, title: b.label || "", s: mins(b.start), e: mins(b.end), start: b.start, end: b.end };
    }).filter(function (b) { return b.s != null && b.e != null && b.e > b.s; });
  }

  function accepted(state) {
    return list(state && state.coming_up).filter(function (e) { return e.accepted === true; }).map(function (e) {
      var start = String(e.start || "");
      return { uid: e.uid, title: e.title || "", date: start.slice(0, 10), time: start.length >= 16 ? start.slice(11, 16) : "",
        location: e.location || "", provenance: e.provenance || null };
    });
  }

  function dueOn(state, date) {
    return list(state && state.list && state.list.rows).filter(function (r) { return typeof r.due === "string" && r.due.slice(0, 10) === date; });
  }

  function dayLoad(state, date) {
    var today = state && state.now ? String(state.now).slice(0, 10) : "";
    if (!state || !today || date < today) { return null; }
    var buckets = list(state.ahead && state.ahead.buckets);
    var last = buckets.length ? buckets[buckets.length - 1].date : today;
    var hours = 0;
    if (date <= last) {
      buckets.forEach(function (b) { if (b.date === date) { hours = Number(b.hours) || 0; } });
    } else {
      list(state.list && state.list.rows).forEach(function (r) {
        if (typeof r.due === "string" && r.due.slice(0, 10) === date) { hours += Number(r.hours) || 0; }
      });
    }
    if (!(hours > 0)) { return null; }
    return hours;
  }

  function lanes(items) {
    var xs = items.slice().sort(function (a, b) { return a.s - b.s || b.e - a.e; });
    var cluster = [], ends = [], end = -Infinity;
    function close() {
      cluster.forEach(function (x) { x.ln = ends.length; });
      cluster = [];
      ends = [];
      end = -Infinity;
    }
    xs.forEach(function (x) {
      if (cluster.length && x.s >= end) { close(); }
      var li = -1;
      for (var k = 0; k < ends.length; k++) { if (ends[k] <= x.s) { li = k; break; } }
      if (li < 0) { li = ends.length; ends.push(x.e); } else { ends[li] = x.e; }
      x.li = li;
      cluster.push(x);
      end = Math.max(end, x.e);
    });
    close();
    return xs;
  }

  window.KnowluCalData = {
    DAY_KEYS: DAY_KEYS, iso: iso, day: day, add: add, monday: monday, mins: mins, slotOf: slotOf, expand: expand,
    unconfirmed: unconfirmed, accepted: accepted, dayLoad: dayLoad, dueOn: dueOn, lanes: lanes
  };
})();
