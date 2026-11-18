import { Extension, type Editor, type Range } from "@tiptap/core";
import { PluginKey } from "@tiptap/pm/state";
import Suggestion from "@tiptap/suggestion";
import { suggestionRenderer, type MenuItem } from "./SuggestionMenu";

interface SlashItem extends MenuItem {
  run: (editor: Editor, range: Range) => void;
}

const ITEMS: SlashItem[] = [
  {
    label: "Heading 1",
    hint: "H1",
    run: (e, r) => e.chain().focus().deleteRange(r).setNode("heading", { level: 1 }).run(),
  },
  {
    label: "Heading 2",
    hint: "H2",
    run: (e, r) => e.chain().focus().deleteRange(r).setNode("heading", { level: 2 }).run(),
  },
  {
    label: "Heading 3",
    hint: "H3",
    run: (e, r) => e.chain().focus().deleteRange(r).setNode("heading", { level: 3 }).run(),
  },
  {
    label: "Bullet list",
    run: (e, r) => e.chain().focus().deleteRange(r).toggleBulletList().run(),
  },
  {
    label: "Numbered list",
    run: (e, r) => e.chain().focus().deleteRange(r).toggleOrderedList().run(),
  },
  {
    label: "To-do list",
    run: (e, r) => e.chain().focus().deleteRange(r).toggleTaskList().run(),
  },
  {
    label: "Quote",
    run: (e, r) => e.chain().focus().deleteRange(r).setBlockquote().run(),
  },
  {
    label: "Code block",
    run: (e, r) => e.chain().focus().deleteRange(r).setCodeBlock().run(),
  },
  {
    label: "Divider",
    run: (e, r) => e.chain().focus().deleteRange(r).setHorizontalRule().run(),
  },
];

/** Typing "/" at the start of a block (or after a space) opens a block-type menu. */
export const SlashCommand = Extension.create({
  name: "slashCommand",

  addProseMirrorPlugins() {
    return [
      Suggestion<SlashItem, SlashItem>({
        editor: this.editor,
        char: "/",
        pluginKey: new PluginKey("slashCommand"),
        items: ({ query }) => {
          const q = query.toLowerCase();
          return ITEMS.filter((i) => i.label.toLowerCase().includes(q));
        },
        command: ({ editor, range, props }) => props.run(editor, range),
        render: suggestionRenderer,
      }),
    ];
  },
});
