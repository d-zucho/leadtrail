import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input, Label, Textarea } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import { CASE_STATUSES, type Case, type CaseInput, type CaseStatus } from "@/lib/api";

export function CaseFormModal({
  initial,
  title,
  submitLabel,
  onSubmit,
  onClose,
}: {
  initial?: Case;
  title: string;
  submitLabel: string;
  onSubmit: (input: CaseInput) => Promise<unknown>;
  onClose: () => void;
}) {
  const [name, setName] = useState(initial?.title ?? "");
  const [summary, setSummary] = useState(initial?.summary ?? "");
  const [status, setStatus] = useState<CaseStatus>(initial?.status ?? "investigating");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await onSubmit({ title: name, summary, status });
      onClose();
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  }

  return (
    <Modal title={title} onClose={onClose}>
      <form onSubmit={submit} className="space-y-4">
        <div>
          <Label>Title</Label>
          <Input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="The waterline contracts"
          />
        </div>
        <div>
          <Label>Summary</Label>
          <Textarea
            rows={3}
            value={summary}
            onChange={(e) => setSummary(e.target.value)}
            placeholder="What are you following, and why?"
          />
        </div>
        <div>
          <Label>Status</Label>
          <select
            value={status}
            onChange={(e) => setStatus(e.target.value as CaseStatus)}
            className="w-full rounded-md border border-line bg-card px-3 py-2 text-sm capitalize outline-none focus:border-forest"
          >
            {CASE_STATUSES.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </div>
        {error && <p className="text-sm text-alert">{error}</p>}
        <div className="flex justify-end gap-2 pt-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" variant="primary" disabled={busy || !name.trim()}>
            {submitLabel}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
