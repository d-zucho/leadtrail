import { Link, useNavigate } from "react-router";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import type { DocKind } from "@/lib/api";
import { timeAgo } from "@/lib/format";
import { useCreateDocument, useDocuments } from "@/lib/queries";

export function DocumentList({ caseId }: { caseId: number }) {
  const { data: docs } = useDocuments(caseId);
  const create = useCreateDocument();
  const navigate = useNavigate();

  async function add(kind: DocKind) {
    const doc = await create.mutateAsync({ caseId, kind });
    navigate(`/cases/${caseId}/docs/${doc.id}`);
  }

  return (
    <section className="mt-10">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="font-display text-3xl">
          Notes &amp; articles{" "}
          <span className="font-mono text-sm text-muted">{docs?.length ?? 0}</span>
        </h2>
        <div className="flex gap-2">
          <Button onClick={() => add("note")} disabled={create.isPending}>
            + New note
          </Button>
          <Button variant="primary" onClick={() => add("article")} disabled={create.isPending}>
            + New article
          </Button>
        </div>
      </div>

      {docs && docs.length === 0 ? (
        <p className="px-1 py-6 text-sm text-muted">
          Nothing written yet. Start a note to think, or an article to tell the story.
        </p>
      ) : (
        <Card className="divide-y divide-line">
          {docs?.map((d) => (
            <Link
              key={d.id}
              to={`/cases/${caseId}/docs/${d.id}`}
              className="flex items-center gap-4 px-4 py-3 hover:bg-sage/40"
            >
              <div className="w-20 shrink-0 font-mono text-[10px] uppercase tracking-wider text-muted">
                {d.kind}
              </div>
              <div className="min-w-0 flex-1">
                <div className="truncate font-medium">{d.title || "Untitled"}</div>
                {d.excerpt && <div className="mt-0.5 truncate text-xs text-muted">{d.excerpt}</div>}
              </div>
              <div className="shrink-0 font-mono text-[11px] text-muted">
                {timeAgo(d.updated_at)}
              </div>
            </Link>
          ))}
        </Card>
      )}
    </section>
  );
}
