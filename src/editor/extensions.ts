import type { Extensions } from "@tiptap/core";
import Mention from "@tiptap/extension-mention";
import Placeholder from "@tiptap/extension-placeholder";
import { TaskItem, TaskList } from "@tiptap/extension-list";
import StarterKit from "@tiptap/starter-kit";
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
}

interface LinkItem extends MenuItem {
  id: string;
  create?: boolean;
}

export function buildExtensions(ctx: EditorContext): Extensions {
  return [
    StarterKit,
    TaskList,
    TaskItem.configure({ nested: true }),
    Placeholder.configure({
      placeholder: "Write something, or press / for blocks and [[ to link a note or source…",
    }),
    SlashCommand,
    Mention.configure({
      HTMLAttributes: { class: "wikilink" },
      renderText: ({ node }) => `[[${node.attrs.label ?? node.attrs.id}]]`,
      renderHTML: ({ options, node }) => [
        "span",
        options.HTMLAttributes,
        `[[${node.attrs.label ?? node.attrs.id}]]`,
      ],
      suggestion: {
        char: "[[",
        // Titles contain spaces, so the query must too.
        allowSpaces: true,
        items: ({ query }): LinkItem[] => {
          const q = query.trim().toLowerCase();
          const matches: LinkItem[] = ctx
            .getTargets()
            .filter((t) => t.label.toLowerCase().includes(q))
            .slice(0, 8);
          const exact = matches.some((t) => t.label.toLowerCase() === q);
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
