// The two required baselines (decision context, e2-brief.md): a majority-class baseline, and an
// honest offline LEXICAL baseline — TF-IDF nearest-neighbour, leave-one-out over the graded
// findings — standing in for the embeddings comparator §4a of the procedure asks for, which this
// machine cannot build (no way to download an embedding model). Named "lexical" everywhere,
// including in the report, never "embeddings": it is a bag-of-words cosine-similarity vote, not a
// learned representation, and the report says so plainly rather than letting the name imply more
// than the method is.

export interface LabeledItem {
  id: string;
  text: string;
  label: string;
}

export interface BaselineResult {
  n: number;
  correct: number;
  accuracy: number;
  /** Wilson 95% confidence interval on `accuracy`. */
  ci95: { lower: number; upper: number };
}

/** Wilson score interval for a binomial proportion — stabler than the normal approximation at small n. */
export function wilsonInterval(
  successes: number,
  n: number,
  z = 1.96,
): { lower: number; upper: number } {
  if (n === 0) return { lower: 0, upper: 0 };
  const p = successes / n;
  const z2 = z * z;
  const denom = 1 + z2 / n;
  const center = p + z2 / (2 * n);
  const margin = z * Math.sqrt((p * (1 - p)) / n + z2 / (4 * n * n));
  return {
    lower: Math.max(0, (center - margin) / denom),
    upper: Math.min(1, (center + margin) / denom),
  };
}

function mode(labels: string[]): string {
  const counts = new Map<string, number>();
  for (const l of labels) counts.set(l, (counts.get(l) ?? 0) + 1);
  let best = labels[0];
  let bestCount = -1;
  // Stable: first-seen label wins a tie, so the result does not depend on Map iteration quirks.
  for (const l of labels) {
    const c = counts.get(l)!;
    if (c > bestCount) {
      bestCount = c;
      best = l;
    }
  }
  return best;
}

/**
 * Leave-one-out majority class: for each item, predicts the mode of every OTHER item's label
 * (not the whole set's mode, which would let the item vote for itself) and scores it. This is the
 * fair form of "always predict the most common class" at small n, and it is what stop rule 3
 * (procedure §4) is measured against.
 */
export function leaveOneOutMajorityClass(items: LabeledItem[]): BaselineResult {
  let correct = 0;
  for (let i = 0; i < items.length; i++) {
    const rest = items.filter((_, j) => j !== i).map((it) => it.label);
    const predicted = mode(rest);
    if (predicted === items[i].label) correct++;
  }
  const n = items.length;
  return { n, correct, accuracy: n === 0 ? 0 : correct / n, ci95: wilsonInterval(correct, n) };
}

// --- TF-IDF lexical nearest-neighbour ---

const STOPWORDS = new Set(
  ("the a an of to in on for and or is are was were be been being this that these those it its " +
    "as at by from with without into over under between so not no never — the plan brief task").split(
      /\s+/,
    ),
);

export function tokenize(text: string): string[] {
  return text
    .toLowerCase()
    .replace(/`[^`]*`/g, " ") // drop inline code spans — file paths/identifiers are near-unique tokens
    .replace(/[^a-z0-9\s]/g, " ")
    .split(/\s+/)
    .filter((t) => t.length > 2 && !STOPWORDS.has(t));
}

function termFrequency(tokens: string[]): Map<string, number> {
  const tf = new Map<string, number>();
  for (const t of tokens) tf.set(t, (tf.get(t) ?? 0) + 1);
  const total = tokens.length || 1;
  for (const [k, v] of tf) tf.set(k, v / total);
  return tf;
}

function inverseDocumentFrequency(docTokens: string[][]): Map<string, number> {
  const df = new Map<string, number>();
  for (const tokens of docTokens) {
    for (const t of new Set(tokens)) df.set(t, (df.get(t) ?? 0) + 1);
  }
  const idf = new Map<string, number>();
  const n = docTokens.length;
  for (const [t, count] of df) idf.set(t, Math.log((n + 1) / (count + 1)) + 1);
  return idf;
}

function tfidfVector(tokens: string[], idf: Map<string, number>): Map<string, number> {
  const tf = termFrequency(tokens);
  const v = new Map<string, number>();
  for (const [t, f] of tf) v.set(t, f * (idf.get(t) ?? 0));
  return v;
}

export function cosineSimilarity(a: Map<string, number>, b: Map<string, number>): number {
  let dot = 0;
  for (const [t, av] of a) {
    const bv = b.get(t);
    if (bv) dot += av * bv;
  }
  let na = 0;
  for (const v of a.values()) na += v * v;
  let nb = 0;
  for (const v of b.values()) nb += v * v;
  if (na === 0 || nb === 0) return 0;
  return dot / (Math.sqrt(na) * Math.sqrt(nb));
}

/**
 * Leave-one-out TF-IDF nearest-neighbour: for each item, the IDF table and every neighbour vector
 * are built from every OTHER item only (never the held-out item), so no information about the
 * held-out item's own text leaks into its own prediction. Predicts the majority label among the
 * `k` most similar remaining items, nearest first breaking ties.
 */
export function leaveOneOutLexicalKNN(items: LabeledItem[], k = 3): BaselineResult {
  const allTokens = items.map((it) => tokenize(it.text));
  let correct = 0;
  for (let i = 0; i < items.length; i++) {
    const restIdx = items.map((_, j) => j).filter((j) => j !== i);
    const restTokens = restIdx.map((j) => allTokens[j]);
    const idf = inverseDocumentFrequency(restTokens);
    const restVectors = restIdx.map((j) => tfidfVector(allTokens[j], idf));
    const queryVector = tfidfVector(allTokens[i], idf);

    const scored = restIdx.map((j, pos) => ({
      j,
      sim: cosineSimilarity(queryVector, restVectors[pos]),
    }));
    scored.sort((a, b) => b.sim - a.sim);
    const top = scored.slice(0, Math.min(k, scored.length));
    const predicted = mode(top.map((t) => items[t.j].label));
    if (predicted === items[i].label) correct++;
  }
  const n = items.length;
  return { n, correct, accuracy: n === 0 ? 0 : correct / n, ci95: wilsonInterval(correct, n) };
}
