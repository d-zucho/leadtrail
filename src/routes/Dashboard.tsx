import { useState } from "react";
import { Link, useNavigate } from "react-router";
import { CaseFormModal } from "@/components/CaseFormModal";
import { QuestionRow } from "@/components/QuestionRow";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { StatTile } from "@/components/ui/StatTile";
import { StatusPill } from "@/components/ui/StatusPill";
import { VerificationBadge } from "@/components/ui/VerificationBadge";
import type { Activity, Case, Lead } from "@/lib/api";
import { greeting, isToday, timeAgo, withinHours } from "@/lib/format";
import {
  useActivity,
  useCases,
  useCreateCase,
  useLeads,
  useQuestions,
  useSources,
} from "@/lib/queries";
import { useUi } from "@/store/ui";

const ACTIVE: Case["status"][] = ["investigating", "verification", "drafting"];

export function Dashboard() {
  const navigate = useNavigate();
  const openCapture = useUi((s) => s.openCapture);
  const createCase = useCreateCase();
  const [creating, setCreating] = useState(false);

  const { data: cases } = useCases();
  const { data: leads } = useLeads();
  const { data: sources } = useSources();
  const { data: questions } = useQuestions();
  const { data: activity } = useActivity(8);

  const activeCases = (cases ?? []).filter((c) => ACTIVE.includes(c.status));
  const openLeads = (leads ?? []).filter((l) => l.status === "new" || l.status === "to_review");
  const toVerify = (sources ?? []).filter(
    (s) => s.verification_status === "unverified" || s.verification_status === "to_review",
  );
  const disputed = (sources ?? []).filter((s) => s.verification_status === "disputed").length;
  const drafting = (cases ?? []).filter((c) => c.status === "drafting").length;
  const openQuestions = (questions ?? []).filter((q) => !q.done);

  // Soonest-due open question per case, for the "Next" line on each card.
  const nextByCase = new Map<number, string>();
  for (const q of openQuestions) if (!nextByCase.has(q.case_id)) nextByCase.set(q.case_id, q.text);

  const today = new Date().toLocaleDateString(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });

  return (
    <div className="mx-auto max-w-6xl">
      <div className="flex items-end justify-between gap-6">
        <div>
          <div className="font-mono text-[11px] uppercase tracking-widest text-muted">{today}</div>
          <h1 className="mt-2 font-display text-6xl leading-none">The research desk</h1>
          <p className="mt-3 text-muted">{greeting()}. Follow the evidence. Keep asking.</p>
        </div>
        <div className="flex gap-2">
          <Button onClick={openCapture}>Quick capture</Button>
          <Button variant="primary" onClick={() => setCreating(true)}>
            + New case file
          </Button>
        </div>
      </div>

      <Card className="mt-8 grid grid-cols-2 divide-line lg:grid-cols-4 lg:divide-x">
        <StatTile
          value={activeCases.length}
          label="Active case files"
          hint={`${activeCases.filter((c) => isToday(c.updated_at)).length} updated today`}
        />
        <StatTile
          value={openLeads.length}
          label="Leads to triage"
          hint={`+${openLeads.filter((l) => withinHours(l.created_at, 24)).length} in the last day`}
        />
        <StatTile
          value={toVerify.length}
          label="Sources to verify"
          hint={disputed > 0 ? `${disputed} disputed` : "None disputed"}
        />
        <StatTile
          value={drafting}
          label="In drafting"
          hint={`${openQuestions.length} open question${openQuestions.length === 1 ? "" : "s"}`}
        />
      </Card>

      <div className="mt-10 grid grid-cols-1 gap-10 lg:grid-cols-[1fr_22rem]">
        <div className="min-w-0 space-y-10">
          <section>
            <SectionHeader title="Open case files" count={activeCases.length} to="/cases" />
            {activeCases.length === 0 ? (
              <Card className="px-6 py-10 text-center">
                <p className="text-muted">No active case files yet.</p>
                <Button variant="primary" className="mt-4" onClick={() => setCreating(true)}>
                  Create a case file
                </Button>
              </Card>
            ) : (
              <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
                {activeCases.slice(0, 4).map((c) => (
                  <CaseCard key={c.id} c={c} next={nextByCase.get(c.id)} />
                ))}
              </div>
            )}
          </section>

          <section>
            <SectionHeader title="New to the desk" count={openLeads.length} to="/inbox" cta="Open inbox" />
            {openLeads.length === 0 ? (
              <p className="px-1 text-sm text-muted">
                Nothing waiting. Press ⌘J anywhere to capture a lead.
              </p>
            ) : (
              <Card className="divide-y divide-line">
                {openLeads.slice(0, 5).map((l) => (
                  <LeadLine key={l.id} lead={l} />
                ))}
              </Card>
            )}
          </section>
        </div>

        <div className="min-w-0 space-y-8">
          <section>
            <SectionHeader title="Questions to follow" count={openQuestions.length} />
            <Card className="divide-y divide-line px-4">
              {openQuestions.length === 0 ? (
                <p className="py-4 text-sm text-muted">
                  No open questions. Add them from a case file.
                </p>
              ) : (
                openQuestions
                  .slice(0, 5)
                  .map((q) => <QuestionRow key={q.id} question={q} showCase />)
              )}
            </Card>
          </section>

          <section>
            <SectionHeader title="Verification queue" count={toVerify.length} to="/sources" />
            <Card className="divide-y divide-line">
              {toVerify.length === 0 ? (
                <p className="p-4 text-sm text-muted">Every source has been looked at.</p>
              ) : (
                toVerify.slice(0, 4).map((s) => (
                  <Link
                    key={s.id}
                    to={`/cases/${s.case_id}`}
                    className="flex items-center justify-between gap-3 px-4 py-3 hover:bg-sage/40"
                  >
                    <div className="min-w-0">
                      <div className="truncate text-sm">{s.title}</div>
                      <div className="font-mono text-[10px] uppercase tracking-wider text-muted">
                        {s.kind} · {s.case_code}
                      </div>
                    </div>
                    <VerificationBadge status={s.verification_status} />
                  </Link>
                ))
              )}
            </Card>
          </section>

          <section>
            <SectionHeader title="Recent activity" />
            {activity && activity.length > 0 ? (
              <ul className="space-y-3">
                {activity.map((a, i) => (
                  <ActivityLine key={`${a.kind}-${a.at}-${i}`} item={a} />
                ))}
              </ul>
            ) : (
              <p className="text-sm text-muted">Activity will show up here as you work.</p>
            )}
          </section>
        </div>
      </div>

      {creating && (
        <CaseFormModal
          title="New case file"
          submitLabel="Create case file"
          onClose={() => setCreating(false)}
          onSubmit={async (input) => {
            const c = await createCase.mutateAsync(input);
            navigate(`/cases/${c.id}`);
          }}
        />
      )}
    </div>
  );
}

function SectionHeader({
  title,
  count,
  to,
  cta = "View all",
}: {
  title: string;
  count?: number;
  to?: string;
  cta?: string;
}) {
  return (
    <div className="mb-3 flex items-baseline justify-between">
      <h2 className="font-display text-3xl">
        {title}
        {count !== undefined && (
          <span className="ml-2 font-mono text-xs text-muted">
            {String(count).padStart(2, "0")}
          </span>
        )}
      </h2>
      {to && (
        <Link to={to} className="text-sm text-muted hover:text-forest">
          {cta} →
        </Link>
      )}
    </div>
  );
}

function CaseCard({ c, next }: { c: Case; next?: string }) {
  return (
    <Link to={`/cases/${c.id}`}>
      <Card className="flex h-full flex-col p-5 transition-shadow hover:shadow-md">
        <div className="mb-3 flex items-center justify-between">
          <span className="font-mono text-xs text-muted">{c.code}</span>
          <StatusPill status={c.status} />
        </div>
        <h3 className="font-display text-3xl leading-tight">{c.title}</h3>
        {c.summary && <p className="mt-2 line-clamp-2 text-muted">{c.summary}</p>}
        <div className="mt-3 flex gap-4 text-xs text-muted">
          <span>
            {c.source_count} source{c.source_count === 1 ? "" : "s"}
          </span>
          <span>
            {c.open_questions} open question{c.open_questions === 1 ? "" : "s"}
          </span>
        </div>
        {next && (
          <div className="mt-4 flex gap-2 rounded-md bg-sage px-3 py-2 text-sm">
            <span className="font-mono text-[10px] uppercase tracking-wider text-muted">Next</span>
            <span className="truncate">{next}</span>
          </div>
        )}
        <div className="mt-auto pt-4 font-mono text-[11px] text-muted">
          Updated {timeAgo(c.updated_at)}
        </div>
      </Card>
    </Link>
  );
}

function LeadLine({ lead }: { lead: Lead }) {
  return (
    <Link to="/inbox" className="flex items-center gap-4 px-4 py-3 hover:bg-sage/40">
      <div className="min-w-0 flex-1">
        <div className="truncate font-medium">{lead.title}</div>
        <div className="mt-0.5 font-mono text-[11px] text-muted">
          {lead.kind} · {timeAgo(lead.created_at)}
        </div>
      </div>
      {lead.case_code && (
        <span className="font-mono text-xs text-muted">{lead.case_code}</span>
      )}
      <span
        className={`rounded px-2 py-0.5 font-mono text-[10px] font-medium tracking-wider ${
          lead.status === "to_review" ? "bg-sand text-[#7a5d1c]" : "bg-line text-muted"
        }`}
      >
        {lead.status === "to_review" ? "TO REVIEW" : "NEW"}
      </span>
    </Link>
  );
}

const VERB: Record<Activity["kind"], string> = {
  case: "Opened case file",
  source: "Added source",
  document: "Updated",
  lead: "Captured lead",
};

function ActivityLine({ item: a }: { item: Activity }) {
  const to =
    a.kind === "document" && a.case_id && a.doc_id
      ? `/cases/${a.case_id}/docs/${a.doc_id}`
      : a.case_id
        ? `/cases/${a.case_id}`
        : "/inbox";
  return (
    <li className="text-sm leading-snug">
      <span className="text-muted">{VERB[a.kind]} </span>
      <Link to={to} className="hover:text-forest">
        {a.title}
      </Link>
      <div className="mt-0.5 font-mono text-[10px] uppercase tracking-wider text-muted">
        {timeAgo(a.at)}
        {a.case_code && ` · ${a.case_code}`}
      </div>
    </li>
  );
}
