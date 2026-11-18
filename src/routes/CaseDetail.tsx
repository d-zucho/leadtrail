import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { AddSourceModal } from "@/components/AddSourceModal";
import { CaseFormModal } from "@/components/CaseFormModal";
import { DocumentList } from "@/components/DocumentList";
import { QuestionsPanel } from "@/components/QuestionsPanel";
import { SourceList } from "@/components/SourceList";
import { Button } from "@/components/ui/Button";
import { StatusPill } from "@/components/ui/StatusPill";
import { timeAgo } from "@/lib/format";
import { useCase, useDeleteCase, useSources, useUpdateCase } from "@/lib/queries";

export function CaseDetail() {
  const id = Number(useParams().id);
  const navigate = useNavigate();
  const { data: c, isLoading, error } = useCase(id);
  const update = useUpdateCase(id);
  const del = useDeleteCase();
  const { data: sources } = useSources(id);
  const [editing, setEditing] = useState(false);
  const [addingSource, setAddingSource] = useState(false);

  if (isLoading) return <p className="text-muted">Loading…</p>;
  if (error || !c) return <p className="text-alert">Case file not found.</p>;

  async function remove() {
    if (!window.confirm(`Delete "${c!.title}"? This cannot be undone.`)) return;
    await del.mutateAsync(id);
    navigate("/cases");
  }

  return (
    <div>
      <Link to="/cases" className="font-mono text-xs text-muted hover:text-ink">
        ← Case files
      </Link>

      <div className="mt-4 flex items-start justify-between gap-6">
        <div>
          <div className="mb-3 flex items-center gap-3">
            <span className="font-mono text-xs text-muted">{c.code}</span>
            <StatusPill status={c.status} />
          </div>
          <h1 className="font-display text-5xl leading-tight">{c.title}</h1>
          <p className="mt-3 max-w-2xl text-muted">
            {c.summary || "No summary yet."}
          </p>
          <div className="mt-3 font-mono text-[11px] text-muted">
            Updated {timeAgo(c.updated_at)}
          </div>
        </div>
        <div className="flex gap-2">
          <Button onClick={() => navigate(`/connections?case=${id}`)}>Connections</Button>
          <Button onClick={() => setEditing(true)}>Edit</Button>
          <Button variant="danger" onClick={remove}>
            Delete
          </Button>
        </div>
      </div>

      <section className="mt-10">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="font-display text-3xl">
            Sources <span className="font-mono text-sm text-muted">{sources?.length ?? 0}</span>
          </h2>
          <Button onClick={() => setAddingSource(true)}>+ Add source</Button>
        </div>
        <SourceList
          sources={sources ?? []}
          empty="No sources yet. Add a document, link, interview, or note."
        />
      </section>

      <DocumentList caseId={id} />

      <QuestionsPanel caseId={id} />

      {addingSource && <AddSourceModal caseId={id} onClose={() => setAddingSource(false)} />}

      {editing && (
        <CaseFormModal
          title="Edit case file"
          submitLabel="Save changes"
          initial={c}
          onClose={() => setEditing(false)}
          onSubmit={(input) => update.mutateAsync(input)}
        />
      )}
    </div>
  );
}
