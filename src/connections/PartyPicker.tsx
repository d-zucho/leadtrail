import { useDeferredValue, useEffect, useState } from "react";
import { ENTITY_KINDS, type PartyRow } from "@/lib/api";
import { usePartySearch } from "@/lib/queries";

const kindLabel = (k: string) => ENTITY_KINDS.find((x) => x.value === k)?.label ?? k;

/**
 * Inline results list shown under the name field. It does not own the input;
 * it listens for ArrowUp/ArrowDown/Enter/Escape on the window (capture phase)
 * so Escape can close the list without closing the surrounding modal.
 */
export function PartyPicker({
  caseId,
  query,
  onPick,
}: {
  caseId: number;
  query: string;
  onPick: (party: PartyRow) => void;
}) {
  const deferred = useDeferredValue(query);
  const { data } = usePartySearch(deferred, caseId, 8);
  const [active, setActive] = useState(-1);
  const [closed, setClosed] = useState(false);

  const blank = query.trim() === "";
  const rows = blank || closed ? [] : (data ?? []);

  // A new query reopens the list and clears the highlight.
  useEffect(() => {
    setClosed(false);
    setActive(-1);
  }, [query]);

  useEffect(() => {
    if (rows.length === 0) return;
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.tagName === "BUTTON")) return;
      const swallow = () => {
        e.preventDefault();
        e.stopPropagation();
      };
      if (e.key === "Escape") {
        swallow();
        setClosed(true);
        setActive(-1);
      } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        swallow();
        const step = e.key === "ArrowDown" ? 1 : -1;
        setActive((cur) => {
          let i = cur;
          for (let n = 0; n < rows.length; n++) {
            i = (i + step + rows.length) % rows.length;
            if (!rows[i].in_this_case) return i;
          }
          return cur;
        });
      } else if (e.key === "Enter" && active >= 0 && rows[active] && !rows[active].in_this_case) {
        swallow();
        onPick(rows[active]);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [rows, active, onPick]);

  if (rows.length === 0) return null;

  return (
    <ul
      role="listbox"
      className="mt-1.5 max-h-56 overflow-y-auto rounded-md border border-line bg-card"
    >
      {rows.map((p, i) => (
        <li key={p.id} role="option" aria-selected={i === active} aria-disabled={p.in_this_case}>
          <button
            type="button"
            disabled={p.in_this_case}
            onMouseEnter={() => !p.in_this_case && setActive(i)}
            onClick={() => onPick(p)}
            className={`w-full px-3 py-2 text-left text-sm disabled:cursor-not-allowed disabled:opacity-60 ${
              i === active ? "bg-sage" : "hover:bg-sage"
            }`}
          >
            <div className="flex items-baseline justify-between gap-2">
              <span className="truncate font-medium">{p.name}</span>
              <span className="shrink-0 font-mono text-[10px] uppercase tracking-wider text-muted">
                {kindLabel(p.kind)}
              </span>
            </div>
            {p.description && <div className="truncate text-xs text-muted">{p.description}</div>}
            {p.in_this_case ? (
              <div className="text-xs text-muted">already in this case</div>
            ) : (
              p.other_case_count != null &&
              p.other_case_count > 0 && (
                <div className="text-xs text-muted">also in {p.other_case_count} other {p.other_case_count === 1 ? "case" : "cases"}</div>
              )
            )}
          </button>
        </li>
      ))}
    </ul>
  );
}
