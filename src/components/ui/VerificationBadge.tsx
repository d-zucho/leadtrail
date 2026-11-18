import type { Verification } from "@/lib/api";

const STYLES: Record<Verification, string> = {
  unverified: "bg-[#f3dcd6] text-alert",
  to_review: "bg-sand text-[#7a5d1c]",
  verified: "bg-sage text-forest",
  disputed: "bg-alert text-white",
};

const LABELS: Record<Verification, string> = {
  unverified: "Unverified",
  to_review: "To review",
  verified: "Verified",
  disputed: "Disputed",
};

export function VerificationBadge({ status }: { status: Verification }) {
  return (
    <span
      className={`rounded px-2 py-0.5 font-mono text-[10px] font-medium uppercase tracking-wider ${STYLES[status]}`}
    >
      {LABELS[status]}
    </span>
  );
}
