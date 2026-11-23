import type { Extensions } from "@tiptap/core";
import Mention from "@tiptap/extension-mention";
import Placeholder from "@tiptap/extension-placeholder";
import { TaskItem, TaskList } from "@tiptap/extension-list";
import StarterKit from "@tiptap/starter-kit";
import { ENTITY_KINDS, type PartyRow } from "@/lib/api";
import { suggestionRenderer, type MenuItem } from "./SuggestionMenu";
import { SlashCommand } from "./slashCommand";

/** A document or source that a [[link]] can point at. */
export interface LinkTarget {
  /** "doc:<id>" or "src:<id>" */
  id: string;
  label: string;
  hint: string;
}

export interface EditorContext {
  /** Always returns the latest targets; read at suggestion time, not at setup time. */
  getTargets: () => LinkTarget[];
  /** Creates a note in the current case and returns its id. */
  createNote: (title: string) => Promise<number>;
  /** Finds people and companies matching the query. */
  searchParties: (query: string) => Promise<PartyRow[]>;
}

interface LinkItem extends MenuItem {
  id: string;
  create?: boolean;
}

export function buildExtensions(ctx: EditorContext): Extensions {
  // Fast typing can resolve searches out of order; only the latest may win.
  let latestRequest = 0;
  return [
    StarterKit,
    TaskList,
    TaskItem.configure({ nested: true }),
    Placeholder.configure({
      placeholder: "Write something, or press / for blocks and [[ to link a note, source, person or company…",
    }),
    SlashCommand,
    Mention.configure({
      HTMLAttributes: { class: "wikilink" },
      renderText: ({ node }) => `[[${node.attrs.label ?? node.attrs.id}]]`,
      renderHTML: ({ options, node }) => [
        "span",
        {
          ...options.HTMLAttributes,
          class: String(node.attrs.id ?? "").startsWith("deleted-party:")
            ? "wikilink wikilink-dead"
            : String(node.attrs.id ?? "").startsWith("party:")
              ? "wikilink wikilink-party"
              : "wikilink",
          "data-type": "mention",
          "data-id": node.attrs.id,
        },
        `[[${node.attrs.label ?? node.attrs.id}]]`,
      ],
      suggestion: {
        char: "[[",
        // Titles contain spaces, so the query must too.
        allowSpaces: true,
        items: async ({ query }): Promise<LinkItem[]> => {
          const request = ++latestRequest;
          const q = query.trim().toLowerCase();
          const matches: LinkItem[] = ctx
            .getTargets()
            .filter((t) => t.label.toLowerCase().includes(q))
            .slice(0, 8);
          const exact = matches.some((t) => t.label.toLowerCase() === q);
          if (q && /[\p{L}\p{N}]/u.test(q)) {
            let parties: PartyRow[] = [];
            try {
              parties = await ctx.searchParties(query.trim());
            } catch {
              // Record rows are optional; note and source rows still work.
            }
            if (request !== latestRequest) return [];
            for (const p of parties.slice(0, 5)) {
              matches.push({
                id: `party:${p.id}`,
                label: p.name,
                hint: ENTITY_KINDS.find((k) => k.value === p.kind)?.label ?? p.kind,
              });
            }
          }
          if (q && !exact) {
            matches.push({
              id: "",
              label: query.trim(),
              hint: "Create note",
              create: true,
            });
          }
          return matches;
        },
        command: async ({ editor, range, props }) => {
          const item = props as unknown as LinkItem;
          let id = item.id;
          if (item.create) {
            try {
              id = `doc:${await ctx.createNote(item.label)}`;
            } catch {
              return;
            }
          }
          editor
            .chain()
            .focus()
            .insertContentAt(range, [
              {
                type: "mention",
                attrs: { id, label: item.label, mentionSuggestionChar: "[[" },
              },
              { type: "text", text: " " },
            ])
            .run();
        },
        render: suggestionRenderer,
      },
    }),
  ];
}
