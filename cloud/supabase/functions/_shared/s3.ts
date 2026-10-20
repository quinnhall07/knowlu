/**
 * S3 by SigV4 for the mail functions (email spec §4.6 step 4 and §5.1's IAM scope; plan T2b). The
 * client does exactly three things to one bucket: **get, put and delete an object**. It has no list
 * operation, and it cannot be made to issue one: every key must sit under `inbound/` (SES's prefix) or
 * `work/` (ours), the two prefixes the bucket's one-day expiry covers, as plain path segments with no
 * query, so no request ever reaches the bucket's root or carries `?list-type`. The IAM user behind it
 * holds the same three permissions and nothing else, so a bug here fails closed at AWS too.
 *
 * **Nothing about a request leaves in an error.** A failure is an `S3Error` carrying only the
 * operation and the HTTP status (0 for a refused key or a transport failure): never the key, the
 * bucket, S3's answer (which echoes the key) or a credential. S3's error bodies are cancelled unread.
 * A handler lets the error reach `asResponse`, which answers a bare 500, so SNS retries a failed put.
 *
 * Requests go to the bucket's virtual host, `https://<bucket>.s3.<region>.amazonaws.com`, through an
 * injected fetcher (production passes the platform's `fetch`; a test serves a fake bucket on
 * 127.0.0.1:0). A redirect is an error, never followed.
 */

export type S3Fetch = (url: string, init?: RequestInit) => Promise<Response>;
export interface S3Config {
  /** `MAIL_BUCKET`. */
  bucket: string;
  /** `AWS_REGION`. */
  region: string;
  /** `AWS_ACCESS_KEY_ID`. */
  accessKeyId: string;
  /** `AWS_SECRET_ACCESS_KEY`. */
  secretAccessKey: string;
}
export interface S3Deps {
  fetch: S3Fetch;
  /** The clock the signature is dated by; the platform's by default. */
  now?: () => Date;
}
export interface S3Client {
  get(key: string): Promise<Uint8Array<ArrayBuffer>>;
  put(key: string, body: Uint8Array): Promise<void>;
  delete(key: string): Promise<void>;
}
export type S3Op = "get" | "put" | "delete";

/** A failed call: the operation and S3's status (0 when no answer came, or the key was refused). */
export class S3Error extends Error {
  constructor(readonly op: S3Op, readonly status: number) {
    super(`s3 ${op} failed (${status})`);
    this.name = "S3Error";
  }
}

/** SES's own cap on a received message is 40 MB; nothing larger is ever read into memory. */
export const MAX_OBJECT_BYTES = 40 * 1024 * 1024;
const FETCH_TIMEOUT_MS = 30_000;
const MAX_KEY_CHARS = 512;
const KEY = /^(?:inbound|work)(?:\/[A-Za-z0-9._-]+)+$/;
const BUCKET = /^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$/;
const REGION = /^[a-z]{2}(?:-[a-z]+)+-\d$/;

/** Is `key` an object under `inbound/` or `work/`, in plain segments, with no `.` or `..` segment? */
export function allowedKey(key: string): boolean {
  if (key.length > MAX_KEY_CHARS || !KEY.test(key)) return false;
  return key.split("/").every((s) => s !== "." && s !== "..");
}

const enc = new TextEncoder();
function hex(b: ArrayBuffer | Uint8Array): string {
  return [...new Uint8Array(b)].map((x) => x.toString(16).padStart(2, "0")).join("");
}
async function sha256Hex(b: Uint8Array): Promise<string> {
  return hex(await crypto.subtle.digest("SHA-256", new Uint8Array(b)));
}
async function hmac(key: Uint8Array, msg: string): Promise<Uint8Array> {
  const k = await crypto.subtle.importKey(
    "raw",
    new Uint8Array(key),
    { name: "HMAC", hash: "SHA-256" },
    false,
    [
      "sign",
    ],
  );
  return new Uint8Array(await crypto.subtle.sign("HMAC", k, enc.encode(msg)));
}

/** RFC 3986 encoding of one path segment, as SigV4 and S3 want it (unreserved characters kept). */
function encodeSegment(s: string): string {
  return encodeURIComponent(s).replace(/[!'()*]/g, (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`);
}

/** `YYYYMMDDTHHMMSSZ`. */
export function amzDate(d: Date): string {
  return d.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}/, "");
}

export interface SigV4Input {
  method: string;
  /** The host the request is sent to; always signed. */
  host: string;
  /** The URI-encoded path, as sent. */
  path: string;
  /** The canonical query string (empty for every call this client makes). */
  query: string;
  /** Every other header to sign, by name; `x-amz-date` and `x-amz-content-sha256` among them. */
  headers: Record<string, string>;
  payloadHash: string;
  amzDate: string;
  region: string;
  accessKeyId: string;
  secretAccessKey: string;
}

/** The `Authorization` header for one S3 request, per AWS's Signature Version 4 (service `s3`). */
export async function sigV4Authorization(i: SigV4Input): Promise<string> {
  const all: Record<string, string> = { host: i.host };
  for (const [k, v] of Object.entries(i.headers)) all[k.toLowerCase()] = v;
  const names = Object.keys(all).sort();
  const canonicalHeaders = names.map((n) => `${n}:${all[n].trim().replace(/\s+/g, " ")}\n`).join("");
  const signedHeaders = names.join(";");
  const canonical = [i.method, i.path, i.query, canonicalHeaders, signedHeaders, i.payloadHash].join("\n");
  const day = i.amzDate.slice(0, 8);
  const scope = `${day}/${i.region}/s3/aws4_request`;
  const toSign = `AWS4-HMAC-SHA256\n${i.amzDate}\n${scope}\n${await sha256Hex(enc.encode(canonical))}`;
  let key = await hmac(enc.encode(`AWS4${i.secretAccessKey}`), day);
  for (const part of [i.region, "s3", "aws4_request"]) key = await hmac(key, part);
  const signature = hex(await hmac(key, toSign));
  return `AWS4-HMAC-SHA256 Credential=${i.accessKeyId}/${scope},SignedHeaders=${signedHeaders},Signature=${signature}`;
}

/** Read a body up to `MAX_OBJECT_BYTES`; anything longer is cancelled and refused. */
async function boundedBytes(res: Response): Promise<Uint8Array<ArrayBuffer> | null> {
  const declared = Number(res.headers.get("content-length") ?? "0");
  if (declared > MAX_OBJECT_BYTES) {
    await res.body?.cancel();
    return null;
  }
  if (res.body === null) return new Uint8Array(0);
  const chunks: Uint8Array[] = [];
  let total = 0;
  const reader = res.body.getReader();
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    total += value.length;
    if (total > MAX_OBJECT_BYTES) {
      await reader.cancel();
      return null;
    }
    chunks.push(value);
  }
  const out = new Uint8Array(total);
  let at = 0;
  for (const c of chunks) {
    out.set(c, at);
    at += c.length;
  }
  return out;
}

/**
 * A client for one bucket. **Throws** a bare `Error` (naming neither value) when the bucket or region
 * is not a plain name, so a misconfigured secret cannot steer requests to another host.
 */
export function s3Client(cfg: S3Config, deps: S3Deps): S3Client {
  if (!BUCKET.test(cfg.bucket) || !REGION.test(cfg.region)) {
    throw new Error("s3 config: the bucket or region is not a plain name");
  }
  const host = `${cfg.bucket}.s3.${cfg.region}.amazonaws.com`;
  const now = deps.now ?? (() => new Date());

  async function send(op: S3Op, method: string, key: string, body?: Uint8Array): Promise<Response> {
    if (!allowedKey(key)) throw new S3Error(op, 0);
    const path = "/" + key.split("/").map(encodeSegment).join("/");
    const payload = body ?? new Uint8Array(0);
    const payloadHash = await sha256Hex(payload);
    const date = amzDate(now());
    const headers: Record<string, string> = { "x-amz-content-sha256": payloadHash, "x-amz-date": date };
    const authorization = await sigV4Authorization({
      method,
      host,
      path,
      query: "",
      headers,
      payloadHash,
      amzDate: date,
      region: cfg.region,
      accessKeyId: cfg.accessKeyId,
      secretAccessKey: cfg.secretAccessKey,
    });
    try {
      return await deps.fetch(`https://${host}${path}`, {
        method,
        headers: { ...headers, authorization },
        body: body === undefined ? undefined : new Uint8Array(body),
        redirect: "manual",
        signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
      });
    } catch {
      // A transport error's message names the URL, and so the bucket and key: drop it.
      throw new S3Error(op, 0);
    }
  }

  /** A non-success answer is cancelled unread (S3's error bodies echo the key) and thrown bare. */
  async function refuse(op: S3Op, res: Response): Promise<never> {
    try {
      await res.body?.cancel();
    } catch {
      // Already closed: nothing to release.
    }
    throw new S3Error(op, res.status);
  }

  return {
    async get(key) {
      const res = await send("get", "GET", key);
      if (res.status !== 200) return await refuse("get", res);
      let bytes: Uint8Array<ArrayBuffer> | null;
      try {
        bytes = await boundedBytes(res);
      } catch {
        throw new S3Error("get", 0);
      }
      if (bytes === null) throw new S3Error("get", 413);
      return bytes;
    },
    async put(key, body) {
      const res = await send("put", "PUT", key, body);
      if (res.status !== 200) return await refuse("put", res);
      await res.body?.cancel();
    },
    async delete(key) {
      const res = await send("delete", "DELETE", key);
      if (res.status !== 204 && res.status !== 200) return await refuse("delete", res);
      await res.body?.cancel();
    },
  };
}
