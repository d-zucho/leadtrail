import { useState } from "react";
import { Link } from "react-router";
import { SourceModal } from "@/components/SourceModal";
import { Card } from "@/components/ui/Card";
import { VerificationBadge } from "@/components/ui/VerificationBadge";
import type { Source, SourceKind } from "@/lib/api";
import { formatBytes, timeAgo } from "@/lib/format";

const KIND_LABEL: Record<SourceKind, string> = {
  file: "FILE",
  link: "LINK",
  note: "NOTE",
  interview: "INTERVIEW",
  record: "PUBLIC RECORD",
};

export function SourceList({
  sources,
  showCase = false,
  empty,
}: {
  sources: Source[];
  showCase?: boolean;
  empty: string;
}) {
  const [openId, setOpenId] = useState<number | null>(null);
  // Look the source up by id so edits made in the modal are reflected after a refetch.
  const selected = sources.find((s) => s.id === openId);

  if (sources.length === 0) {
    return <p className="px-1 py-6 text-sm text-muted">{empty}</p>;
  }

  return (
    <>
      <Card className="divide-y divide-line">
        {sources.map((s) => (
          <div
            key={s.id}
            className="flex cursor-pointer items-center gap-4 px-4 py-3 hover:bg-sage/40"
            onClick={() => setOpenId(s.id)}
          >
            <div className="w-24 shrink-0 font-mono text-[10px] tracking-wider text-muted">
              {KIND_LABEL[s.kind]}
            </div>
            <div className="min-w-0 flex-1">
              <div className="truncate font-medium">{s.title}</div>
              <div className="mt-0.5 truncate text-xs text-muted">
                {s.kind === "file" && s.size != null && `${formatBytes(s.size)} · `}
                Added {timeAgo(s.created_at)}
              </div>
            </div>
            {showCase && (
              <Link
                to={`/cases/${s.case_id}`}
                onClick={(e) => e.stopPropagation()}
                className="font-mono text-xs text-muted hover:text-forest"
              >
                {s.case_code}
              </Link>
            )}
            <VerificationBadge status={s.verification_status} />
          </div>
        ))}
      </Card>
      {selected && <SourceModal source={selected} onClose={() => setOpenId(null)} />}
    </>
  );
}
