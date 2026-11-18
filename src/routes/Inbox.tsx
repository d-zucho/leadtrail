import { useState } from "react";
import { Link } from "react-router";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import type { Lead, LeadStatus } from "@/lib/api";
import { timeAgo } from "@/lib/format";
import {
  useCases,
  useDeleteLead,
  useLeads,
  usePromoteLead,
  useUpdateLead,
} from "@/lib/queries";
import { useUi } from "@/store/ui";

type Tab = "open" | "promoted" | "dismissed";

const TABS: { value: Tab; label: string }[] = [
  { value: "open", label: "Open" },
  { value: "promoted", label: "In a case file" },
  { value: "dismissed", label: "Dismissed" },
];

const STATUS_STYLE: Record<LeadStatus, string> = {
  new: "bg-line text-muted",
  to_review: "bg-sand text-[#7a5d1c]",
  promoted: "bg-sage text-forest",
  dismissed: "bg-line text-muted",
};

const STATUS_LABEL: Record<LeadStatus, string> = {
  new: "NEW",
  to_review: "TO REVIEW",
  promoted: "IN CASE",
  dismissed: "DISMISSED",
};

const inTab = (l: Lead, tab: Tab) =>
  tab === "open" ? l.status === "new" || l.status === "to_review" : l.status === tab;

export function Inbox() {
  const { data: leads, isLoading, error } = useLeads();
  const [tab, setTab] = useState<Tab>("open");
  const openCapture = useUi((s) => s.openCapture);

  const visible = (leads ?? []).filter((l) => inTab(l, tab));
  const count = (t: Tab) => (leads ?? []).filter((l) => inTab(l, t)).length;

  return (
    <div>
      <div className="flex items-end justify-between">
        <div>
          <h1 className="font-display text-5xl">Lead inbox</h1>
          <p className="mt-2 text-muted">Capture first. Connect later.</p>
        </div>
        <Button variant="primary" onClick={openCapture}>
          + Capture
        </Button>
      </div>

      <div className="my-6 flex gap-1">
        {TABS.map((t) => (
          <button
            key={t.value}
            onClick={() => setTab(t.value)}
            className={`rounded-md px-3 py-1.5 text-sm ${
              tab === t.value ? "bg-forest text-white" : "text-muted hover:bg-sage"
            }`}
          >
            {t.label} <span className="font-mono text-xs opacity-70">{count(t.value)}</span>
          </button>
        ))}
      </div>

      {isLoading && <p className="text-muted">Loading…</p>}
      {error && <p className="text-alert">{String(error)}</p>}

      {leads && visible.length === 0 && (
        <p className="px-1 py-6 text-sm text-muted">
          {tab === "open" ? "Inbox zero. Press ⌘J anywhere to capture a lead." : "Nothing here."}
        </p>
      )}

      {visible.length > 0 && (
        <Card className="divide-y divide-line">
          {visible.map((l) => (
            <LeadRow key={l.id} lead={l} />
          ))}
        </Card>
      )}
    </div>
  );
}

function LeadRow({ lead }: { lead: Lead }) {
  const { data: cases } = useCases();
  const update = useUpdateLead();
  const promote = usePromoteLead();
  const del = useDeleteLead();
  const [error, setError] = useState<string | null>(null);

  const patch = (changes: Partial<{ status: LeadStatus; case_id: number | null }>) =>
    update.mutateAsync({
      id: lead.id,
      input: {
        title: lead.title,
        url: lead.url ?? undefined,
        body: lead.body,
        case_id: lead.case_id,
        status: lead.status,
        ...changes,
      },
    });

  async function run(fn: () => Promise<unknown>) {
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    }
  }

  const isOpen = lead.status === "new" || lead.status === "to_review";

  return (
    <div className="px-4 py-3">
      <div className="flex items-start gap-4">
        <div className="min-w-0 flex-1">
          <div className="font-medium">{lead.title}</div>
          {lead.body && <div className="mt-0.5 line-clamp-2 text-sm text-muted">{lead.body}</div>}
          <div className="mt-1 font-mono text-[11px] text-muted">
            {lead.kind} · {timeAgo(lead.created_at)}
            {lead.status === "promoted" && lead.case_id && (
              <>
                {" · "}
                <Link to={`/cases/${lead.case_id}`} className="hover:text-forest">
                  {lead.case_code}
                </Link>
              </>
            )}
          </div>
        </div>
        <span
          className={`rounded px-2 py-0.5 font-mono text-[10px] font-medium tracking-wider ${STATUS_STYLE[lead.status]}`}
        >
          {STATUS_LABEL[lead.status]}
        </span>
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        {isOpen && (
          <>
            <select
              value={lead.case_id ?? ""}
              onChange={(e) =>
                run(() => patch({ case_id: e.target.value ? Number(e.target.value) : null }))
              }
              className="rounded-md border border-line bg-card px-2 py-1 text-xs outline-none focus:border-forest"
            >
              <option value="">Choose a case file…</option>
              {cases?.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.code} · {c.title}
                </option>
              ))}
            </select>
            <Button
              variant="primary"
              className="px-3 py-1 text-xs"
              disabled={!lead.case_id || promote.isPending}
              onClick={() => run(() => promote.mutateAsync({ id: lead.id, caseId: lead.case_id! }))}
            >
              Add to case as source
            </Button>
            <Button
              variant="ghost"
              className="px-3 py-1 text-xs"
              onClick={() =>
                run(() => patch({ status: lead.status === "new" ? "to_review" : "new" }))
              }
            >
              {lead.status === "new" ? "Flag for review" : "Unflag"}
            </Button>
            <Button
              variant="ghost"
              className="px-3 py-1 text-xs"
              onClick={() => run(() => patch({ status: "dismissed" }))}
            >
              Dismiss
            </Button>
          </>
        )}
        {lead.status === "dismissed" && (
          <Button
            variant="ghost"
            className="px-3 py-1 text-xs"
            onClick={() => run(() => patch({ status: "new" }))}
          >
            Restore
          </Button>
        )}
        <Button
          variant="ghost"
          className="ml-auto px-3 py-1 text-xs text-alert"
          onClick={() => {
            if (window.confirm("Delete this lead? This cannot be undone.")) {
              run(() => del.mutateAsync(lead.id));
            }
          }}
        >
          Delete
        </Button>
      </div>
      {error && <p className="mt-2 text-xs text-alert">{error}</p>}
    </div>
  );
}
