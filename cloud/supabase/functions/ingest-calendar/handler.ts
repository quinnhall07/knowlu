// GET /ingest-calendar?name=<feed name> — the account's calendar, as ICS.
//
// Two sources, one shape:
//   * `google_calendar` — the Google grant's own events for the engine's window, rendered as ICS.
//     Read-only, `calendar.readonly` only, and the grant is the same one Gmail uses when the
//     student has taken that step too (§11a).
//   * `calendar_ics` — the secret iCal address the wizard captured, fetched server-side. This is
//     the path C1 ships and the fallback while the OAuth scope is unverified; serving it here as
//     well means the address stops needing to be in the vault once the account has it.
//
// The secret address is a capability URL, exactly like the LMS one: it never appears in a reply,
// an error body or a log line.
//
// `accepts=series` (comma-separated, unknown words ignored; `name=google` only) adds a `series`
// field: the recurring events on the student's own calendars, reduced in memory and returned,
// never stored or logged (commitment model spec §4.1, §9). Without it the reply is byte-for-byte
// what it was before series existed.
import { decryptString } from "../_shared/crypto.ts";
import type { Entitle } from "../_shared/judge_handler.ts";

/** The engine's own window (`calfeed::HORIZON_DAYS`). Asking for more would be discarded. */
export const HORIZON_DAYS = 28;
export const CAL_USER_AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Knowlu/1.0";

/**
 * The `?name=` -> source mapping, and the ONLY place it is decided (ruling R-X-9).
 *
 *   `personal` (and the default) -> the account's `sources` row where `kind = 'calendar_ics'`
 *   `google`                     -> the account's `google_accounts` row, if its `scopes` carry
 *                                   `calendar.readonly`
 *   anything else                -> a 404 that names the name
 *
 * **`sources` has no `name` column** — its primary key is `(account_id, kind)` — and **the Google
 * calendar is not a `sources` row at all**: a grant has no URL, and both `url_ciphertext` and
 * `url_iv` are `not null`. `google_calendar` exists in C1's check constraint as a reserved value
 * so neither stream ever has to edit the other's constraint, and C2 never writes a row of that
 * kind. The two names are the two `calendars:` entries the vault carries, which is what makes the
 * name in the query string and the name in the vault the same word.
 */
export const CALENDAR_NAMES = { personal: "calendar_ics", google: "google_accounts" } as const;

/** One `calendarList` entry, as Google sends it (only the fields read here). */
export interface GoogleCalendarEntry {
  id: string;
  accessRole?: string;
  hidden?: boolean;
  primary?: boolean;
}

/** One `events.list` item, as Google sends it (only the fields read here). */
export interface GoogleEvent {
  id: string;
  status?: string;
  summary?: string;
  location?: string;
  description?: string;
  eventType?: string;
  recurringEventId?: string;
  recurrence?: string[];
  start?: { dateTime?: string; date?: string };
  end?: { dateTime?: string; date?: string };
  attendees?: Array<{ self?: boolean; email?: string; responseStatus?: string }>;
}

/** One raw `events.list` page. */
export interface GooglePage {
  items: GoogleEvent[];
  nextPageToken?: string;
}

/**
 * Read methods only — no write, insert, upsert or persist member, ever (Limited Use, spec §9):
 * `calendar.readonly` data is fetched, reduced and returned, never kept. `handler_test.ts` pins
 * the member list.
 */
export interface CalendarDeps {
  /** The account's `sources` row of kind `calendar_ics`, or null. */
  personalSource(accountId: string): Promise<{ ciphertext: string; iv: string } | null>;
  /** A fresh Google access token, or null when the grant is gone or lacks the calendar scope. */
  calendarTokenFor(accountId: string): Promise<string | null>;
  /** Google Calendar's events for the window, already normalised. */
  googleEvents(accessToken: string, from: Date, to: Date): Promise<
    Array<{
      uid: string;
      summary: string;
      start: string;
      end: string;
      allDay: boolean;
    }>
  >;
  /** The account's calendar list. Every series call takes the budget's signal. */
  calendarList(accessToken: string, signal: AbortSignal): Promise<GoogleCalendarEntry[]>;
  /** One page of `events.list` with `singleEvents=true` for one calendar. */
  seriesInstances(
    accessToken: string,
    calendarId: string,
    from: Date,
    to: Date,
    pageToken: string | undefined,
    signal: AbortSignal,
  ): Promise<GooglePage>;
  /** One page of `events.list` with `singleEvents=false` (the series masters) for one calendar. */
  seriesMasters(
    accessToken: string,
    calendarId: string,
    from: Date,
    to: Date,
    pageToken: string | undefined,
    signal: AbortSignal,
  ): Promise<GooglePage>;
  fetchText(url: string): Promise<string>;
  encKey(): Promise<CryptoKey>;
  now(): Date;
}

function stamp(value: string, allDay: boolean): string {
  // `YYYYMMDD` for an all-day event, `YYYYMMDDTHHMMSSZ` otherwise — the two forms
  // `calfeed::parse_dt` reads. Google returns RFC 3339, so this is a reshape and not a parse.
  const d = new Date(value);
  const pad = (n: number) => String(n).padStart(2, "0");
  const day = `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}`;
  if (allDay) return day;
  return `${day}T${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}${pad(d.getUTCSeconds())}Z`;
}

function escapeIcs(text: string): string {
  return text.replace(/\\/g, "\\\\").replace(/;/g, "\\;").replace(/,/g, "\\,").replace(/\r?\n/g, "\\n");
}

export function toIcs(
  events: Array<{ uid: string; summary: string; start: string; end: string; allDay: boolean }>,
): string {
  const lines = ["BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//Knowlu//ingest-calendar//EN"];
  for (const e of events) {
    lines.push("BEGIN:VEVENT");
    lines.push(`UID:${escapeIcs(e.uid)}`);
    lines.push(`SUMMARY:${escapeIcs(e.summary)}`);
    lines.push(e.allDay ? `DTSTART;VALUE=DATE:${stamp(e.start, true)}` : `DTSTART:${stamp(e.start, false)}`);
    lines.push(e.allDay ? `DTEND;VALUE=DATE:${stamp(e.end, true)}` : `DTEND:${stamp(e.end, false)}`);
    lines.push("END:VEVENT");
  }
  lines.push("END:VCALENDAR");
  // CRLF, because that is what RFC 5545 says and what every other feed the engine reads uses.
  return lines.join("\r\n") + "\r\n";
}

// ---- Series (commitment model spec §4.1) ----

/** Series gathering's own wall-clock budget, after `ics` is built. */
export const SERIES_BUDGET_MS = 5_000;
/** The series window: from 24 hours before now (plan review M7) to the horizon after it. */
export const SERIES_LOOKBACK_MS = 86_400_000;
export const MAX_CALENDARS = 10;
export const MAX_PAGES = 5;
export const MAX_SERIES = 100;
export const MAX_INSTANCES = 40;
export const MAX_TEXT = 200;
const KEPT_RECURRENCE = /^(RRULE|EXDATE|RDATE)[:;]/;

/** Thrown (and aborted with) when the series budget runs out; its class name is the log line. */
export class SeriesBudgetExceeded extends Error {}

export interface SeriesItem {
  calendar: string;
  id: string;
  title: string;
  location: string;
  description: string;
  event_type: string;
  first?: string;
  recurrence?: string[];
  instances: Array<{ start: string; end: string }>;
}

export interface Series {
  calendars_read: string[];
  items: SeriesItem[];
}

/** At most `MAX_TEXT` characters (code points, so a surrogate pair is never split). */
function cut(text: string | undefined): string {
  const chars = Array.from(text ?? "");
  return chars.length <= MAX_TEXT ? chars.join("") : chars.slice(0, MAX_TEXT).join("");
}

/** RFC 3339 with an offset -> `YYYY-MM-DDTHH:MM:SSZ`. */
function utc(value: string): string {
  return new Date(value).toISOString().replace(/\.\d{3}Z$/, "Z");
}

/** `google:` + 16 hex of sha256(account id, newline, calendar id): an identity and nothing else (R5). */
export async function calendarKey(accountId: string, calendarId: string): Promise<string> {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(`${accountId}\n${calendarId}`),
  );
  const hex = Array.from(new Uint8Array(digest)).map((b) => b.toString(16).padStart(2, "0")).join("");
  return `google:${hex.slice(0, 16)}`;
}

/** `primary`, then owned and not hidden, by id; the first `MAX_CALENDARS`. */
export function pickCalendars(entries: GoogleCalendarEntry[]): string[] {
  const primary = entries.find((e) => e.primary === true)?.id ?? "primary";
  const owned = entries
    .filter((e) => e.primary !== true && e.id !== primary && e.accessRole === "owner" && e.hidden !== true)
    .map((e) => e.id)
    .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  return [primary, ...owned].slice(0, MAX_CALENDARS);
}

function keptInstance(e: GoogleEvent): boolean {
  return e.recurringEventId !== undefined && e.recurringEventId !== "" &&
    e.start?.dateTime !== undefined && e.end?.dateTime !== undefined &&
    e.status !== "cancelled" &&
    !(e.attendees ?? []).some((a) => a.self === true && a.responseStatus === "declined");
}

/** One calendar's series, in series id order; a series past `MAX_INSTANCES` is dropped. */
function reduceCalendar(key: string, instances: GoogleEvent[], masters: GoogleEvent[]): SeriesItem[] {
  const bySeries = new Map<string, GoogleEvent[]>();
  for (const e of instances.filter(keptInstance)) {
    const list = bySeries.get(e.recurringEventId as string) ?? [];
    list.push(e);
    bySeries.set(e.recurringEventId as string, list);
  }
  const masterById = new Map<string, GoogleEvent>();
  for (const m of masters) {
    if (Array.isArray(m.recurrence) && m.status !== "cancelled") masterById.set(m.id, m);
  }
  const items: SeriesItem[] = [];
  for (const id of [...bySeries.keys()].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))) {
    const seen = bySeries.get(id) as GoogleEvent[];
    if (seen.length > MAX_INSTANCES) continue; // cannot be weekly-eligible
    const master = masterById.get(id);
    const src = master ?? seen[0];
    const location = cut(src.location);
    // Key order is the wire order of spec §4.1's example; `first` and `recurrence` exist only when
    // the master came back (without one the device rules the series ineligible).
    const fromMaster: Pick<SeriesItem, "first" | "recurrence"> = {};
    if (master !== undefined) {
      const first = master.start?.dateTime ?? master.start?.date;
      if (first !== undefined) fromMaster.first = first;
      fromMaster.recurrence = (master.recurrence as string[]).filter((line) => KEPT_RECURRENCE.test(line));
    }
    items.push({
      calendar: key,
      id,
      title: cut(src.summary),
      location,
      // R4: a description only stands in for a missing location.
      description: location === "" ? cut(src.description) : "",
      event_type: seen[0].eventType ?? "default",
      ...fromMaster,
      instances: seen
        .map((e) => ({ start: utc(e.start?.dateTime as string), end: utc(e.end?.dateTime as string) }))
        .sort((a, b) => (a.start < b.start ? -1 : a.start > b.start ? 1 : 0)),
    });
  }
  return items;
}

/**
 * Gathers the series under `signal`. `check` runs after every Google call and throws once the
 * handler's own clock says the budget is spent; the signal ends a call that never returns.
 */
async function gatherSeries(
  deps: CalendarDeps,
  token: string,
  accountId: string,
  from: Date,
  to: Date,
  signal: AbortSignal,
  check: () => void,
): Promise<Series> {
  const entries = await deps.calendarList(token, signal);
  check();
  const calendars = pickCalendars(entries);
  const pages = async (
    call: CalendarDeps["seriesInstances"],
    calendarId: string,
  ): Promise<GoogleEvent[] | null> => {
    const items: GoogleEvent[] = [];
    let pageToken: string | undefined = undefined;
    for (let page = 0; page < MAX_PAGES; page++) {
      const got: GooglePage = await call(token, calendarId, from, to, pageToken, signal);
      check();
      items.push(...(got.items ?? []));
      if (got.nextPageToken === undefined || got.nextPageToken === "") return items;
      pageToken = got.nextPageToken;
    }
    return null; // a sixth page would be needed: not read completely
  };
  const read = await Promise.all(calendars.map(async (calendarId) => {
    const [instances, masters] = await Promise.all([
      pages(deps.seriesInstances.bind(deps), calendarId),
      pages(deps.seriesMasters.bind(deps), calendarId),
    ]);
    const key = await calendarKey(accountId, calendarId);
    return instances === null || masters === null
      ? null
      : { key, items: reduceCalendar(key, instances, masters) };
  }));
  // Complete or not at all: a calendar that ran past its pages, or whose series would cross the
  // cap, is left out of `calendars_read` and none of its items are sent (review I6).
  const series: Series = { calendars_read: [], items: [] };
  for (const calendar of read) {
    if (calendar === null) continue;
    if (series.items.length + calendar.items.length > MAX_SERIES) continue;
    series.calendars_read.push(calendar.key);
    series.items.push(...calendar.items);
  }
  return series;
}

/**
 * Series within the budget, or null. Null on overrun or on any failure: a series problem never
 * costs the day's busy time. The only log line is the exception's class name (spec §9).
 */
async function seriesWithinBudget(
  deps: CalendarDeps,
  token: string,
  accountId: string,
  now: Date,
  to: Date,
  budgetMs: number,
): Promise<Series | null> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(new SeriesBudgetExceeded()), budgetMs);
  const started = deps.now().getTime();
  const check = () => {
    if (deps.now().getTime() - started > budgetMs) throw new SeriesBudgetExceeded();
  };
  const from = new Date(now.getTime() - SERIES_LOOKBACK_MS);
  const work = gatherSeries(deps, token, accountId, from, to, controller.signal, check);
  work.catch(() => {}); // abandoned at the budget, it may still settle later
  const aborted = new Promise<never>((_, reject) => {
    controller.signal.addEventListener("abort", () => reject(controller.signal.reason), { once: true });
  });
  aborted.catch(() => {});
  try {
    return await Promise.race([work, aborted]);
  } catch (e) {
    console.error(`ingest-calendar series: ${e instanceof Error ? e.constructor.name : "unknown"}`);
    return null;
  } finally {
    clearTimeout(timer);
    controller.abort(new SeriesBudgetExceeded()); // tell any call still running to stop
  }
}

export interface CalendarOptions {
  /** Series gathering's budget; `SERIES_BUDGET_MS` unless a test shortens it. */
  seriesBudgetMs?: number;
}

export function calendarHandler(
  entitle: Entitle,
  deps: CalendarDeps,
  options: CalendarOptions = {},
): (req: Request) => Promise<Response> {
  const budgetMs = options.seriesBudgetMs ?? SERIES_BUDGET_MS;
  return async (req: Request): Promise<Response> => {
    if (req.method !== "GET") return Response.json({ error: "GET only" }, { status: 405 });
    try {
      const { account_id } = await entitle(req);
      const params = new URL(req.url).searchParams;
      const name = params.get("name") ?? "personal";
      // A comma-separated capability list; unknown words are ignored (spec §4.1).
      const accepts = (params.get("accepts") ?? "").split(",").map((word) => word.trim());
      if (!(name in CALENDAR_NAMES)) {
        return Response.json({ error: `no calendar named '${name}' for this account` }, { status: 404 });
      }
      if (name === "personal") {
        const source = await deps.personalSource(account_id);
        if (source === null) {
          return Response.json({ error: "no calendar named 'personal' for this account" }, { status: 404 });
        }
        const url = await decryptString(await deps.encKey(), source.ciphertext, source.iv);
        let ics: string;
        try {
          ics = await deps.fetchText(url);
        } catch {
          // The cause is swallowed: a transport error's text quotes the secret address.
          return Response.json({ error: "the calendar could not be fetched" }, { status: 502 });
        }
        if (!ics.includes("BEGIN:VCALENDAR")) {
          return Response.json({ error: "the calendar is not an ICS response" }, { status: 502 });
        }
        return Response.json({ ics, source: "calendar_ics" });
      }
      const token = await deps.calendarTokenFor(account_id);
      if (token === null) {
        // Not connected, revoked, or connected for Gmail only — all one answer, and the device
        // degrades to `using snapshot` rather than losing the day's capacity model.
        return Response.json({ error: "the Google calendar is not connected" }, { status: 409 });
      }
      const from = deps.now();
      const to = new Date(from.getTime() + HORIZON_DAYS * 86_400_000);
      let ics: string;
      try {
        ics = toIcs(await deps.googleEvents(token, from, to));
      } catch {
        return Response.json({ error: "the calendar could not be fetched" }, { status: 502 });
      }
      // Without `series` in `accepts` the reply is byte-for-byte what it was before series existed
      // (an old engine keeps working). With it, `series` is added only when gathered in full.
      const series = accepts.includes("series")
        ? await seriesWithinBudget(deps, token, account_id, from, to, budgetMs)
        : null;
      if (series === null) return Response.json({ ics, source: "google_calendar" });
      return Response.json({ ics, source: "google_calendar", series });
    } catch (e) {
      if (e instanceof Response) return e;
      console.error(`ingest-calendar: ${e instanceof Error ? e.constructor.name : "unknown"}`);
      return Response.json({ error: "calendar failed" }, { status: 500 });
    }
  };
}
