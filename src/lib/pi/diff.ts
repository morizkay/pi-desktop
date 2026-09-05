export type DiffLine = {
  kind: "add" | "remove" | "context" | "meta";
  text: string;
  old?: number;
  next?: number;
};
export type DiffHunk = { header: string; lines: DiffLine[]; patch: string };
export function parseHunks(diff: string): DiffHunk[] {
  const hunks: DiffHunk[] = [];
  let current: DiffHunk | undefined;
  let old = 0,
    next = 0;
  for (const line of diff.split("\n")) {
    const match = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (match) {
      old = Number(match[1]);
      next = Number(match[2]);
      current = { header: line, lines: [], patch: line };
      hunks.push(current);
    } else if (current && /^[ +\\-]/.test(line)) {
      current.patch += `\n${line}`;
      const kind =
        line[0] === "+"
          ? "add"
          : line[0] === "-"
            ? "remove"
            : line[0] === " "
              ? "context"
              : "meta";
      current.lines.push({
        kind,
        text: line,
        old: kind === "remove" || kind === "context" ? old++ : undefined,
        next: kind === "add" || kind === "context" ? next++ : undefined,
      });
    } else if (line.startsWith("diff --git")) current = undefined;
  }
  return hunks;
}
export function quoteHunk(
  path: string,
  hunk: DiffHunk,
  staged: boolean,
): string {
  return `${path} (${staged ? "staged" : "working tree"})\n${hunk.patch}`;
}
