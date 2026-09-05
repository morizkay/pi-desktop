import { test, expect } from "@playwright/test";
import { validateResult, parseForkMessages } from "../src/lib/pi/protocol";
import { parseActiveEntries, messageTextFromPi } from "../src/lib/pi/messages";
import { parseHunks, quoteHunk } from "../src/lib/pi/diff";

test("shared transport rejects failed and cancelled envelopes", () => {
  expect(() =>
    validateResult({ type: "response", success: false, error: "denied" }),
  ).toThrow("denied");
  expect(() =>
    validateResult({
      type: "response",
      success: true,
      data: { cancelled: true },
    }),
  ).toThrow("cancelled");
  expect(() => validateResult({ type: "response" })).toThrow("missing success");
  expect(validateResult({ type: "response", success: true })).toEqual({
    type: "response",
    success: true,
  });
});
test("fork schemas use stable entry IDs, not duplicate prompt text", () => {
  const response = {
    type: "response",
    success: true,
    data: {
      messages: [
        { entryId: "first", text: "same" },
        { entryId: "second", text: "same" },
      ],
    },
  };
  expect(parseForkMessages(response).map((m) => m.entryId)).toEqual([
    "first",
    "second",
  ]);
  const messages = parseActiveEntries({
    type: "response",
    success: true,
    data: {
      leafId: "second",
      entries: [
        {
          type: "message",
          id: "first",
          parentId: null,
          message: { role: "user", content: "same" },
        },
        {
          type: "message",
          id: "abandoned",
          parentId: "first",
          message: { role: "assistant", content: "wrong branch" },
        },
        {
          type: "message",
          id: "second",
          parentId: "first",
          message: { role: "user", content: "same" },
        },
      ],
    },
  });
  expect(messages.map((m) => m.entryId)).toEqual(["first", "second"]);
  expect(
    messageTextFromPi({
      content: [
        { type: "text", text: "hello\u2028world" },
        { type: "thinking", thinking: "hidden" },
      ],
    }),
  ).toBe("hello\u2028world");
});
test("hunk parser tracks added, removed and context line numbers", () => {
  const hunks = parseHunks(
    "--- a/test.ts\n+++ b/test.ts\n@@ -8,2 +8,3 @@ function\n-old\n+new\n+extra\n context\n\\ No newline at end of file\n@@ -20 +21 @@\n-a\n+b\n",
  );
  expect(hunks).toHaveLength(2);
  expect(hunks[0].lines.map((l) => [l.kind, l.old, l.next])).toEqual([
    ["remove", 8, undefined],
    ["add", undefined, 8],
    ["add", undefined, 9],
    ["context", 9, 10],
    ["meta", undefined, undefined],
  ]);
  expect(quoteHunk("test.ts", hunks[0], true)).toContain(
    "test.ts (staged)\n@@ -8,2 +8,3 @@",
  );
  expect(parseHunks("Binary files a/logo.png and b/logo.png differ")).toEqual(
    [],
  );
});
