import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { api, type Hit, type HitKind } from "@/lib/api";
import { useUi } from "@/store/ui";

interface Item {
  key: string;
  label: string;
  hint: string;
  snippet?: string;
  run: () => void;
}

const KIND_LABEL: Record<HitKind, string> = {
  case: "Case",
  source: "Source",
  lead: "Lead",
  document: "Note",
  question: "Question",
  entity: "Entity",
};

function destination(h: Hit): string {
  switch (h.kind) {
    case "case":
      return `/cases/${h.ref_id}`;
    case "document":
      return `/cases/${h.case_id}/docs/${h.ref_id}`;
    case "lead":
      return "/inbox";
    case "entity":
      return `/connections?case=${h.case_id}`;
    default:
      return `/cases/${h.case_id}`;
  }
}

/** Splits an FTS snippet on its \u0001/\u0002 markers into plain and matched runs. */
function Highlighted({ text }: { text: string }) {
  const parts = text.split(/[\u0001\u0002]/);
  return (
    <>
      {parts.map((p, i) =>
        i % 2 === 1 ? (
          <mark key={i} className="rounded bg-sand px-0.5 text-ink">
            {p}
          </mark>
        ) : (
          <span key={i}>{p}</span>
        ),
      )}
    </>
  );
}

export function CommandPalette() {
  const open = useUi((s) => s.paletteOpen);
  if (!open) return null;
  return <Palette />;
}

function Palette() {
  const close = useUi((s) => s.closePalette);
  const openCapture = useUi((s) => s.openCapture);
  const navigate = useNavigate();
  const [input, setInput] = useState("");
  const [term, setTerm] = useState("");
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    const t = setTimeout(() => setTerm(input.trim()), 120);
    return () => clearTimeout(t);
  }, [input]);

  const { data: hits = [], isFetching } = useQuery({
    queryKey: ["search", term],
    queryFn: () => api.search(term),
    enabled: term.length > 0,
    staleTime: 0,
  });

  const items: Item[] = useMemo(() => {
    const go = (to: string) => () => {
      close();
      navigate(to);
    };
    const commands: Item[] = [
      { key: "c:dash", label: "Go to Research desk", hint: "Go", run: go("/") },
      { key: "c:cases", label: "Go to Case files", hint: "Go", run: go("/cases") },
      { key: "c:inbox", label: "Go to Lead inbox", hint: "Go", run: go("/inbox") },
      { key: "c:sources", label: "Go to Source library", hint: "Go", run: go("/sources") },
      { key: "c:graph", label: "Go to Connections", hint: "Go", run: go("/connections") },
      {
        key: "c:capture",
        label: "Quick capture a lead",
        hint: "⌘J",
        run: () => {
          close();
          openCapture();
        },
      },
    ];
    const q = input.trim().toLowerCase();
    const matching = q ? commands.filter((c) => c.label.toLowerCase().includes(q)) : commands;
    // Results belong to `term`; hide them while the input has moved on.
    const results: Item[] =
      term && term === input.trim()
        ? hits.map((h) => ({
            key: `${h.kind}:${h.ref_id}`,
            label: h.title || "Untitled",
            hint: [KIND_LABEL[h.kind], h.case_code].filter(Boolean).join(" · "),
            snippet: h.snippet,
            run: go(destination(h)),
          }))
        : [];
    return [...results, ...matching];
  }, [hits, input, term, close, navigate, openCapture]);

  useEffect(() => setActive(0), [items.length, term]);
  useEffect(() => {
    listRef.current?.children[active]?.scrollIntoView({ block: "nearest" });
  }, [active]);

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((a) => Math.min(a + 1, items.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((a) => Math.max(a - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      items[active]?.run();
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-ink/30 px-4 pt-[14vh]"
      onMouseDown={(e) => e.target === e.currentTarget && close()}
    >
      <div
        role="dialog"
        aria-label="Search"
        className="w-full max-w-xl overflow-hidden rounded-xl border border-line bg-paper shadow-xl"
        onKeyDown={onKeyDown}
      >
        <input
          autoFocus
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Search notes, sources, leads, entities…"
          className="w-full border-b border-line bg-card px-4 py-3 text-base outline-none"
        />
        <ul ref={listRef} className="max-h-[50vh] overflow-y-auto p-2">
          {items.map((it, i) => (
            <li key={it.key}>
              <button
                onMouseMove={() => setActive(i)}
                onClick={it.run}
                className={`w-full rounded-md px-3 py-2 text-left ${i === active ? "bg-sage" : ""}`}
              >
                <div className="flex items-baseline justify-between gap-3">
                  <span className="truncate text-sm font-medium">{it.label}</span>
                  <span className="shrink-0 font-mono text-[10px] uppercase tracking-wider text-muted">
                    {it.hint}
                  </span>
                </div>
                {it.snippet && (
                  <div className="mt-0.5 truncate text-xs text-muted">
                    <Highlighted text={it.snippet} />
                  </div>
                )}
              </button>
            </li>
          ))}
          {items.length === 0 && (
            <li className="px-3 py-6 text-center text-sm text-muted">
              {isFetching ? "Searching…" : `Nothing found for “${term}”.`}
            </li>
          )}
        </ul>
      </div>
    </div>
  );
}
