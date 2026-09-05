export type JsonRecord = Record<string, unknown>;
export function record(value: unknown): JsonRecord {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as JsonRecord)
    : {};
}
export function validateResult<T>(raw: T): T {
  const value = record(raw);
  if (value.success === false)
    throw new Error(
      String(
        value.error ||
          value.stderr ||
          value.stdout ||
          "Pi rejected the request",
      ),
    );
  if (record(value.data).cancelled === true)
    throw new Error(
      "Request cancelled by a Pi extension. Nothing was changed.",
    );
  if (value.type === "response" && value.success !== true)
    throw new Error("Invalid Pi response: missing success flag");
  return raw;
}
export function rpcData(raw: unknown): JsonRecord {
  validateResult(raw);
  const value = record(raw);
  return record(value.type === "response" ? value.data : raw);
}
export type PiModel = { provider: string; id: string; name: string };
export function parseModels(raw: unknown): PiModel[] {
  const models = rpcData(raw).models;
  return Array.isArray(models)
    ? models.flatMap((m) => {
        const v = record(m);
        return typeof v.provider === "string" && typeof v.id === "string"
          ? [
              {
                provider: v.provider,
                id: v.id,
                name: typeof v.name === "string" ? v.name : v.id,
              },
            ]
          : [];
      })
    : [];
}
export type ForkMessage = { entryId: string; text: string };
export function parseForkMessages(raw: unknown): ForkMessage[] {
  const messages = rpcData(raw).messages;
  return Array.isArray(messages)
    ? messages.flatMap((m) => {
        const v = record(m);
        return typeof v.entryId === "string" && typeof v.text === "string"
          ? [{ entryId: v.entryId, text: v.text }]
          : [];
      })
    : [];
}
