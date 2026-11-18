export function Placeholder({ title }: { title: string }) {
  return (
    <div>
      <h1 className="font-display text-5xl">{title}</h1>
      <p className="mt-3 text-muted">Coming soon.</p>
    </div>
  );
}
