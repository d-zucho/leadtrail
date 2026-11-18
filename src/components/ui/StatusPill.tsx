import type { CaseStatus } from "@/lib/api";

const STYLES: Record<CaseStatus, string> = {
  investigating: "bg-sage text-forest",
  verification: "bg-sand text-[#7a5d1c]",
  drafting: "bg-[#e3e6f1] text-[#3a4a85]",
  published: "bg-forest text-white",
  archived: "bg-line text-muted",
};

export function StatusPill({ status }: { status: CaseStatus }) {
  return (
    <span
      className={`rounded px-2 py-0.5 font-mono text-[10px] font-medium uppercase tracking-wider ${STYLES[status]}`}
    >
      {status}
    </span>
  );
}
