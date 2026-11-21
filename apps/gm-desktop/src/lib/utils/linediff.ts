/**
 * A line-by-line diff, for the conflict dialog's two-column view.
 *
 * `[PROPOSAL]` ADR-0024 §3.4.3: a suggestion is *always* presented as a
 * diff — the ours side on the left, the suggestion on the right — and
 * not as a side-by-side pair of texts and not as a silent replacement.
 *
 * The algorithm is the usual longest-common-subsequence walk, with the
 * two cheap normalisations that keep it cheap on real inputs: the
 * common prefix and the common suffix are stripped before the table is
 * built, and a pair of inputs whose product is too large falls back to
 * "everything removed, everything added" rather than allocating a table
 * that would exhaust the webview.
 *
 * No dependency: the repository forbids new packages, and a line diff is
 * not worth one.
 */

export type DiffOp = 'same' | 'remove' | 'add';

export interface DiffRow {
  op: DiffOp;
  /** The left column's line, or null when the row has no left side. */
  left: string | null;
  /** The right column's line, or null when the row has no right side. */
  right: string | null;
}

/**
 * The largest table the LCS walk may build.
 *
 * `[PROPOSAL]` 2000 by 2000 lines is four million cells, which is a few
 * tens of megabytes — too much for a webview to hold on top of the
 * application. Beyond it the diff degrades to "the whole block changed",
 * which is still an honest reading of a conflict region.
 */
const MAX_TABLE_CELLS = 1_000_000;

/** Split text into lines, dropping the terminators. */
function splitLines(text: string): string[] {
  if (text === '') return [];
  const lines = text.split('\n');
  // `[FACT]` A trailing newline is a terminator, not an empty last line.
  if (lines.length > 0 && lines[lines.length - 1] === '') lines.pop();
  return lines;
}

/**
 * The diff of two texts, line by line.
 *
 * The rows are in order and cover both inputs: every line of the left
 * text appears once, and so does every line of the right text.
 */
export function diffLines(left: string, right: string): DiffRow[] {
  const a = splitLines(left);
  const b = splitLines(right);

  // Common prefix.
  let start = 0;
  while (start < a.length && start < b.length && a[start] === b[start]) start += 1;

  // `[FACT]` Common suffix, without overlapping the prefix.
  let endA = a.length;
  let endB = b.length;
  while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) {
    endA -= 1;
    endB -= 1;
  }

  const midA = a.slice(start, endA);
  const midB = b.slice(start, endB);

  const rows: DiffRow[] = [];
  for (let i = 0; i < start; i += 1) rows.push({ op: 'same', left: a[i], right: b[i] });

  if (midA.length * midB.length > MAX_TABLE_CELLS) {
    for (const line of midA) rows.push({ op: 'remove', left: line, right: null });
    for (const line of midB) rows.push({ op: 'add', left: null, right: line });
  } else {
    rows.push(...lcsRows(midA, midB));
  }

  for (let i = 0; i < a.length - endA; i += 1) {
    rows.push({ op: 'same', left: a[endA + i], right: b[endB + i] });
  }
  return rows;
}

/**
 * The LCS walk over two line arrays.
 *
 * The table is filled from the bottom-right, so the table itself is the
 * length table: `table[i][j]` is the length of the longest common
 * subsequence of `a[i..]` and `b[j..]`.
 */
function lcsRows(a: string[], b: string[]): DiffRow[] {
  const rows: DiffRow[] = [];
  if (a.length === 0 && b.length === 0) return rows;

  const table: number[][] = Array.from({ length: a.length + 1 }, () =>
    new Array<number>(b.length + 1).fill(0)
  );
  for (let i = a.length - 1; i >= 0; i -= 1) {
    for (let j = b.length - 1; j >= 0; j -= 1) {
      table[i][j] =
        a[i] === b[j] ? table[i + 1][j + 1] + 1 : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }

  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      rows.push({ op: 'same', left: a[i], right: b[j] });
      i += 1;
      j += 1;
    } else if (table[i + 1][j] >= table[i][j + 1]) {
      rows.push({ op: 'remove', left: a[i], right: null });
      i += 1;
    } else {
      rows.push({ op: 'add', left: null, right: b[j] });
      j += 1;
    }
  }
  while (i < a.length) {
    rows.push({ op: 'remove', left: a[i], right: null });
    i += 1;
  }
  while (j < b.length) {
    rows.push({ op: 'add', left: null, right: b[j] });
    j += 1;
  }
  return rows;
}
