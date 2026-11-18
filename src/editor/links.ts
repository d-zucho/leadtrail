import type { JSONContent } from "@tiptap/core";
import type { LinkRef } from "@/lib/api";

const ID_RE = /^(doc|src):(\d+)$/;

/** Mention ids are stored as "doc:<id>" or "src:<id>". */
export function parseTargetId(id: string | null | undefined) {
  const m = id ? ID_RE.exec(id) : null;
  if (!m) return null;
  return { type: m[1] === "doc" ? ("document" as const) : ("source" as const), id: Number(m[2]) };
}

/** Collects the [[links]] in a document, one per distinct target. */
export function extractLinks(json: JSONContent, selfId: number): LinkRef[] {
  const seen = new Map<string, LinkRef>();
  const walk = (node: JSONContent) => {
    if (node.type === "mention") {
      const t = parseTargetId(node.attrs?.id);
      if (t && !(t.type === "document" && t.id === selfId)) {
        const key = `${t.type}:${t.id}`;
        if (!seen.has(key)) {
          seen.set(key, { dst_type: t.type, dst_id: t.id, label: String(node.attrs?.label ?? "") });
        }
      }
    }
    node.content?.forEach(walk);
  };
  walk(json);
  return [...seen.values()];
}
