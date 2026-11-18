export function StatTile({
  value,
  label,
  hint,
}: {
  value: number;
  label: string;
  hint?: string;
}) {
  return (
    <div className="flex items-center gap-4 px-6 py-5">
      <div className="font-display text-5xl leading-none">
        {String(value).padStart(2, "0")}
      </div>
      <div>
        <div className="font-mono text-[10px] uppercase tracking-widest text-muted">
          {label}
        </div>
        {hint && <div className="mt-1 text-xs text-muted">{hint}</div>}
      </div>
    </div>
  );
}
