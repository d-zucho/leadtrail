import { forwardRef, useEffect, useImperativeHandle, useState } from "react";
import { ReactRenderer } from "@tiptap/react";
import type { SuggestionKeyDownProps, SuggestionProps } from "@tiptap/suggestion";

export interface MenuItem {
  label: string;
  hint?: string;
}

export interface MenuHandle {
  onKeyDown: (e: KeyboardEvent) => boolean;
}

interface MenuProps {
  items: MenuItem[];
  command: (item: MenuItem) => void;
}

const MenuList = forwardRef<MenuHandle, MenuProps>(function MenuList({ items, command }, ref) {
  const [selected, setSelected] = useState(0);

  // New results mean the old highlighted index no longer points at the same thing.
  useEffect(() => setSelected(0), [items]);

  useImperativeHandle(ref, () => ({
    onKeyDown(e) {
      if (items.length === 0) return false;
      if (e.key === "ArrowDown") {
        setSelected((s) => (s + 1) % items.length);
        return true;
      }
      if (e.key === "ArrowUp") {
        setSelected((s) => (s + items.length - 1) % items.length);
        return true;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        command(items[selected] ?? items[0]);
        return true;
      }
      return false;
    },
  }));

  if (items.length === 0) return null;

  return (
    <div className="max-h-72 w-72 overflow-y-auto rounded-lg border border-line bg-card p-1 shadow-lg">
      {items.map((item, i) => (
        <button
          key={`${item.label}-${i}`}
          type="button"
          // mousedown, not click: keep focus in the editor so the suggestion stays open.
          onMouseDown={(e) => {
            e.preventDefault();
            command(item);
          }}
          onMouseEnter={() => setSelected(i)}
          className={`flex w-full items-center justify-between gap-3 rounded-md px-2.5 py-1.5 text-left text-sm ${
            i === selected ? "bg-sage" : ""
          }`}
        >
          <span className="truncate">{item.label}</span>
          {item.hint && (
            <span className="shrink-0 font-mono text-[10px] uppercase tracking-wider text-muted">
              {item.hint}
            </span>
          )}
        </button>
      ))}
    </div>
  );
});

/** Renders a suggestion list in a floating element positioned under the caret. */
export function suggestionRenderer() {
  let component: ReactRenderer<MenuHandle, MenuProps> | null = null;

  const place = (props: SuggestionProps<any, any>) => {
    const rect = props.clientRect?.();
    if (!rect || !component) return;
    const el = component.element as HTMLElement;
    el.style.position = "fixed";
    el.style.left = `${rect.left}px`;
    el.style.top = `${rect.bottom + 6}px`;
    el.style.zIndex = "60";
  };

  const close = () => {
    component?.element.remove();
    component?.destroy();
    component = null;
  };

  return {
    onStart: (props: SuggestionProps<any, any>) => {
      component = new ReactRenderer(MenuList, { props, editor: props.editor });
      document.body.appendChild(component.element);
      place(props);
    },
    onUpdate: (props: SuggestionProps<any, any>) => {
      // Async pickers re-emit with no items while loading; keep the open list.
      if (!(props.loading && props.items.length === 0)) component?.updateProps(props);
      place(props);
    },
    onKeyDown: (props: SuggestionKeyDownProps) => {
      if (props.event.key === "Escape") {
        close();
        return true;
      }
      return component?.ref?.onKeyDown(props.event) ?? false;
    },
    onExit: close,
  };
}
