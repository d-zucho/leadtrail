import { Link } from "react-router";
import { Card } from "@/components/ui/Card";
import { DOC_STATUSES, type DocStatus, type QueueItem } from "@/lib/api";
import { timeAgo } from "@/lib/format";
import { useQueue, useSetDocumentStatus } from "@/lib/queries";

const SELECT =
  "mt-2 w-full rounded-md border border-line bg-card px-2 py-1 text-xs outline-none focus:border-forest";

function QueueCard({ item }: { item: QueueItem }) {
  const move = useSetDocumentStatus();
  return (
    <Card className="p-3">
      <Link
        to={`/cases/${item.case_id}/docs/${item.id}`}
        className="block font-display text-lg leading-snug hover:text-forest"
      >
        {item.title || "Untitled"}
      </Link>
      <div className="mt-1 font-mono text-[10px] uppercase tracking-wider text-muted">
        {item.case_code} · {item.words.toLocaleString()} words
      </div>
      <div className="font-mono text-[10px] text-muted">
        {item.status === "published" && item.published_at
          ? `Published ${timeAgo(item.published_at)}`
          : `Edited ${timeAgo(item.updated_at)}`}
      </div>
      <select
        aria-label={`Move ${item.title || "Untitled"}`}
        value={item.status}
        disabled={move.isPending}
        onChange={(e) => move.mutate({ id: item.id, status: e.target.value as DocStatus })}
        className={SELECT}
      >
        {DOC_STATUSES.map((s) => (
          <option key={s.value} value={s.value}>
            {s.label}
          </option>
        ))}
      </select>
      {move.error && <p className="mt-1 text-xs text-alert">{String(move.error)}</p>}
    </Card>
  );
}

export function Publication() {
  const { data: items, isLoading, error } = useQueue();

  if (isLoading) return <p className="text-muted">Loading…</p>;
  if (error) return <p className="text-alert">{String(error)}</p>;

  return (
    <div>
      <h1 className="font-display text-5xl">Publication queue</h1>
      <p className="mt-3 max-w-2xl text-muted">
        Every article across your case files, by where it stands. Open one to export it as Markdown,
        HTML or PDF.
      </p>
      {items && items.length === 0 && (
        <p className="mt-8 text-sm text-muted">
          No articles yet. Create one from a case file and it will show up here.
        </p>
      )}
      <div className="mt-8 grid grid-cols-4 gap-5">
        {DOC_STATUSES.map((col) => {
          const inCol = (items ?? []).filter((i) => i.status === col.value);
          return (
            <section key={col.value}>
              <h2 className="mb-3 font-mono text-[11px] uppercase tracking-widest text-muted">
                {col.label} <span className="ml-1">{inCol.length}</span>
              </h2>
              <div className="space-y-3">
                {inCol.map((i) => (
                  <QueueCard key={i.id} item={i} />
                ))}
              </div>
            </section>
          );
        })}
      </div>
    </div>
  );
}
