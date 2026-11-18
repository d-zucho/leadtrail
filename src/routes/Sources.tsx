import { useState } from "react";
import { SourceList } from "@/components/SourceList";
import { VERIFICATIONS, type Verification } from "@/lib/api";
import { useSources } from "@/lib/queries";

type Filter = "all" | "needs_attention" | Verification;

export function Sources() {
  const { data: sources, isLoading, error } = useSources();
  const [filter, setFilter] = useState<Filter>("all");

  const filtered = (sources ?? []).filter((s) => {
    if (filter === "all") return true;
    if (filter === "needs_attention") {
      return s.verification_status === "unverified" || s.verification_status === "to_review";
    }
    return s.verification_status === filter;
  });

  const tabs: { value: Filter; label: string }[] = [
    { value: "all", label: "All" },
    { value: "needs_attention", label: "Needs attention" },
    ...VERIFICATIONS,
  ];

  return (
    <div>
      <h1 className="font-display text-5xl">Source library</h1>
      <p className="mt-2 text-muted">
        Every source across your case files. Open a case to add new ones.
      </p>

      <div className="my-6 flex flex-wrap gap-1">
        {tabs.map((t) => (
          <button
            key={t.value}
            onClick={() => setFilter(t.value)}
            className={`rounded-md px-3 py-1.5 text-sm ${
              filter === t.value ? "bg-forest text-white" : "text-muted hover:bg-sage"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {isLoading && <p className="text-muted">Loading…</p>}
      {error && <p className="text-alert">{String(error)}</p>}
      {sources && (
        <SourceList
          sources={filtered}
          showCase
          empty={
            sources.length === 0
              ? "No sources yet. Open a case file and add your first one."
              : "Nothing matches this filter."
          }
        />
      )}
    </div>
  );
}
