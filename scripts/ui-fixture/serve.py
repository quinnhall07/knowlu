"""Serve the new page with a stub host, for the fixture harness (v0.4.0 plan T5c, note §8 M12).

Binds 127.0.0.1 on a port the OS picks (port 0) and serves:

  /index.html?set=<set>       the shipped document from app/static/next, with one <script> added at the top of
                              <head>: the stub below, which loads before prefs.js
  /<path>                     any other file under app/static/next, unchanged
  /__fixture__/stub.js        the stub host (STUB, below)
  /__fixture__/envelopes/...  the staged envelopes (envelopes/<set>-{today,all}.json, <set>-answers.json)

The stub stands in for Tauri's window.__TAURI__: core.invoke answers from the named envelope set and records every
call in window.__fixture.calls; event.emit records in window.__fixture.events; window.getCurrentWindow() is a window
that does nothing. It freezes Date at 2026-08-28T09:00 local (the surface oracle's pinned now), so every date word is
the same on any day. Nothing here is shipped, and nothing is written under app/static.

    python scripts/ui-fixture/serve.py [--set base-courses]   # prints the URL; Ctrl+C stops it
"""
import argparse
import http.server
import pathlib
import threading
import urllib.parse

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent.parent
# The page's root. T23 (the swap) moves this one line to app/static.
ROOT = REPO / "app" / "static" / "next"
ENVELOPES = HERE / "envelopes"

STUB = r"""
(function () {
  "use strict";
  var RealDate = Date;
  var FIXED = new RealDate(2026, 7, 28, 9, 0, 0, 0).getTime();
  function FixedDate() {
    if (!(this instanceof FixedDate)) { return new RealDate(FIXED).toString(); }
    if (!arguments.length) { return new RealDate(FIXED); }
    var args = [null].concat(Array.prototype.slice.call(arguments));
    return new (Function.prototype.bind.apply(RealDate, args))();
  }
  FixedDate.prototype = RealDate.prototype;
  FixedDate.now = function () { return FIXED; };
  FixedDate.parse = RealDate.parse;
  FixedDate.UTC = RealDate.UTC;
  window.Date = FixedDate;

  var SET = new URLSearchParams(location.search).get("set") || "base-courses";
  var F = window.__fixture = { set: SET, calls: [], events: [], window: [] };
  var cache = {};
  function load(name) {
    if (!cache[name]) {
      cache[name] = fetch("__fixture__/envelopes/" + name + ".json").then(function (r) { return r.ok ? r.json() : null; });
    }
    return cache[name];
  }
  function copy(x) { return x == null ? x : JSON.parse(JSON.stringify(x)); }
  var DEFAULTS = {
    launch_state: { ok: true, error: null, mode: "console" },
    grades_status: { ok: true, available: false, reason: "not connected", connected: false, signed_out: false, host: null, fetched_at: null, hidden: false },
    mail_status: { ok: true, error: null, mailboxes: [] },
    privacy_status: { ok: true, error: null, current: "2026-10-20", recorded: "2026-10-20", needs_accept: false },
    ui_event: { ok: true, error: null },
    mark_seen: { ok: true, error: null, seen_at: "2026-08-28T14:00:00Z" }
  };
  function answer(cmd, args) {
    return load(SET + "-answers").then(function (staged) {
      if (staged && Object.prototype.hasOwnProperty.call(staged, cmd)) { return copy(staged[cmd]); }
      var view = args && typeof args.view === "string" ? args.view : null;
      if (cmd === "state" || (view && cmd !== "ui_event")) {
        return load(SET + "-" + (view === "all" ? "all" : "today")).then(function (env) {
          if (!env) { return { ok: false, error: "no envelope staged for " + SET, state: null }; }
          return copy(env);
        });
      }
      if (Object.prototype.hasOwnProperty.call(DEFAULTS, cmd)) { return copy(DEFAULTS[cmd]); }
      return { ok: false, error: "not staged in the fixture harness: " + cmd };
    });
  }
  function invoke(cmd, args) {
    F.calls.push({ cmd: cmd, args: copy(args || {}) });
    return answer(cmd, args || {});
  }
  function quiet(name) { return function () { F.window.push(name); return Promise.resolve(); }; }
  var win = {
    minimize: quiet("minimize"),
    toggleMaximize: quiet("toggleMaximize"),
    close: quiet("close"),
    setTheme: quiet("setTheme"),
    isMaximized: function () { F.window.push("isMaximized"); return Promise.resolve(false); },
    onResized: function () { return Promise.resolve(function () {}); }
  };
  window.__TAURI__ = {
    core: { invoke: invoke },
    event: {
      emit: function (name) { F.events.push(name); return Promise.resolve(); },
      listen: function () { return Promise.resolve(function () {}); }
    },
    window: { getCurrentWindow: function () { return win; } }
  };
})();
"""

STUB_TAG = '<script src="__fixture__/stub.js"></script>'

# Content types by suffix, fixed here: the Windows registry's guesses vary by machine.
TYPES = {
    ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".css": "text/css; charset=utf-8",
    ".json": "application/json; charset=utf-8", ".svg": "image/svg+xml", ".png": "image/png",
    ".ttf": "font/ttf", ".woff": "font/woff", ".woff2": "font/woff2", ".md": "text/plain; charset=utf-8", ".txt": "text/plain; charset=utf-8",
}


class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def send_bytes(self, body, ctype):
        self.send_response(200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        path = urllib.parse.urlsplit(self.path).path
        if path in ("/", "/index.html"):
            html = (ROOT / "index.html").read_text(encoding="utf-8")
            html = html.replace("<head>", "<head>\n" + STUB_TAG, 1)
            return self.send_bytes(html.encode("utf-8"), "text/html; charset=utf-8")
        if path == "/__fixture__/stub.js":
            return self.send_bytes(STUB.encode("utf-8"), "text/javascript; charset=utf-8")
        if path.startswith("/__fixture__/envelopes/"):
            name = path.rsplit("/", 1)[1]
            target = ENVELOPES / name
            if name.endswith(".json") and "/" not in name and "\\" not in name and target.is_file():
                return self.send_bytes(target.read_bytes(), "application/json; charset=utf-8")
            if name.endswith("-answers.json"):
                return self.send_bytes(b"{}", "application/json; charset=utf-8")  # a set with no staged answers
            return self.send_error(404)
        if path == "/favicon.ico":
            return self.send_bytes(b"", "image/x-icon")  # the browser's own probe; the app has no favicon
        target = (ROOT / urllib.parse.unquote(path.lstrip("/"))).resolve()
        if ROOT.resolve() not in target.parents or not target.is_file():
            return self.send_error(404)
        ctype = TYPES.get(target.suffix.lower(), "application/octet-stream")
        return self.send_bytes(target.read_bytes(), ctype)


class Server(http.server.ThreadingHTTPServer):
    # The page asks for about sixty files at once; the stdlib's backlog of 5 refuses some of them.
    request_queue_size = 128
    daemon_threads = True


def start():
    """Start the server on 127.0.0.1:0 in a daemon thread; returns (server, base URL)."""
    server = Server(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    host, port = server.server_address[:2]
    return server, "http://%s:%d/" % (host, port)


def main():
    ap = argparse.ArgumentParser(description="Serve the new page with the fixture stub on 127.0.0.1.")
    ap.add_argument("--set", default="base-courses", help="the envelope set to answer from")
    args = ap.parse_args()
    server, base = start()
    print("%sindex.html?set=%s#home" % (base, args.set), flush=True)
    try:
        threading.Event().wait()
    except KeyboardInterrupt:
        server.shutdown()


if __name__ == "__main__":
    main()
