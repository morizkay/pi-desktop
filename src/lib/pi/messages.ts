import { record, rpcData } from "./protocol";

export type ChatMessage = {
  id: string;
  role: "user" | "assistant" | "system";
  text: string;
  timestamp?: number;
  entryId?: string;
  error?: string;
};
export function messageTextFromPi(msg: unknown): string {
  const content = record(msg).content;
  if (typeof content === "string") return content;
  return Array.isArray(content)
    ? content
        .map((p) => {
          const part = record(p);
          return part.type === "text" && typeof part.text === "string"
            ? part.text
            : "";
        })
        .filter(Boolean)
        .join("\n")
    : "";
}
export function chatMessage(
  msg: unknown,
  id: string,
  entryId?: string,
): ChatMessage | null {
  const m = record(msg);
  if (m.role !== "user" && m.role !== "assistant" && m.role !== "system")
    return null;
  return {
    id,
    role: m.role,
    text: messageTextFromPi(m),
    entryId,
    timestamp: typeof m.timestamp === "number" ? m.timestamp : undefined,
    error: typeof m.errorMessage === "string" ? m.errorMessage : undefined,
  };
}
export function parseGetMessagesResponse(raw: unknown): ChatMessage[] {
  const messages = rpcData(raw).messages;
  return Array.isArray(messages)
    ? messages.flatMap((msg, i) => {
        const parsed = chatMessage(msg, `message-${i}`);
        return parsed ? [parsed] : [];
      })
    : [];
}
// get_entries includes abandoned branches. Follow leaf → parent, never text-match duplicate prompts.
export function parseActiveEntries(raw: unknown): ChatMessage[] {
  const data = rpcData(raw);
  if (!Array.isArray(data.entries)) return [];
  const entries = new Map(
    data.entries.map((e) => {
      const v = record(e);
      return [v.id, v] as const;
    }),
  );
  const out: ChatMessage[] = [];
  const seen = new Set<unknown>();
  let id = data.leafId;
  while (typeof id === "string" && !seen.has(id)) {
    seen.add(id);
    const e = entries.get(id);
    if (!e) break;
    if (e.type === "message") {
      const m = chatMessage(e.message, id, id);
      if (m) out.push(m);
    }
    id = e.parentId;
  }
  return out.reverse();
}
