import { useState } from "react";
import { Link, useNavigate } from "react-router";
import { CaseFormModal } from "@/components/CaseFormModal";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { StatusPill } from "@/components/ui/StatusPill";
import { timeAgo } from "@/lib/format";
import { useCases, useCreateCase } from "@/lib/queries";

export function Cases() {
  const { data: cases, isLoading, error } = useCases();
  const create = useCreateCase();
  const navigate = useNavigate();
  const [creating, setCreating] = useState(false);

  return (
    <div>
      <div className="mb-8 flex items-end justify-between">
        <div>
          <h1 className="font-display text-5xl">Case files</h1>
          <p className="mt-2 text-muted">Everything you are following, in one place.</p>
        </div>
        <Button variant="primary" onClick={() => setCreating(true)}>
          + New case file
        </Button>
      </div>

      {isLoading && <p className="text-muted">Loading…</p>}
      {error && <p className="text-alert">{String(error)}</p>}

      {cases && cases.length === 0 && (
        <Card className="px-8 py-14 text-center">
          <h2 className="font-display text-3xl">No case files yet</h2>
          <p className="mx-auto mt-2 max-w-sm text-muted">
            Start a case file to gather sources, evidence, and notes around a story.
          </p>
          <Button variant="primary" className="mt-6" onClick={() => setCreating(true)}>
            Create your first case file
          </Button>
        </Card>
      )}

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        {cases?.map((c) => (
          <Link key={c.id} to={`/cases/${c.id}`}>
            <Card className="h-full p-5 transition-shadow hover:shadow-md">
              <div className="mb-3 flex items-center justify-between">
                <span className="font-mono text-xs text-muted">{c.code}</span>
                <StatusPill status={c.status} />
              </div>
              <h2 className="font-display text-3xl leading-tight">{c.title}</h2>
              {c.summary && (
                <p className="mt-2 line-clamp-2 text-muted">{c.summary}</p>
              )}
              <div className="mt-4 flex gap-4 font-mono text-[11px] text-muted">
                <span>{c.source_count} sources</span>
                <span>{c.document_count} notes</span>
                <span>{c.open_questions} open questions</span>
                <span className="ml-auto">Updated {timeAgo(c.updated_at)}</span>
              </div>
            </Card>
          </Link>
        ))}
      </div>

      {creating && (
        <CaseFormModal
          title="New case file"
          submitLabel="Create case file"
          onClose={() => setCreating(false)}
          onSubmit={async (input) => {
            const c = await create.mutateAsync(input);
            navigate(`/cases/${c.id}`);
          }}
        />
      )}
    </div>
  );
}
