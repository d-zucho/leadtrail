import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Label, Textarea } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import type { LeadInput } from "@/lib/api";
import { useCases, useCreateLead } from "@/lib/queries";
import { useUi } from "@/store/ui";

const URL_RE = /^https?:\/\/\S+$/i;

/** First line becomes the title; the rest is the body. A bare URL becomes a link lead. */
export function parseCapture(text: string): LeadInput | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  if (URL_RE.test(trimmed)) return { title: trimmed, kind: "link", url: trimmed };
  const [first, ...rest] = trimmed.split("\n");
  return { title: first.trim(), kind: "note", body: rest.join("\n").trim() };
}

export function QuickCapture() {
  const open = useUi((s) => s.captureOpen);
  const close = useUi((s) => s.closeCapture);
  if (!open) return null;
  return <CaptureForm onClose={close} />;
}

function CaptureForm({ onClose }: { onClose: () => void }) {
  const [text, setText] = useState("");
  const [caseId, setCaseId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const { data: cases } = useCases();
  const create = useCreateLead();

  async function submit(e?: React.FormEvent) {
    e?.preventDefault();
    const parsed = parseCapture(text);
    if (!parsed || busy) return;
    setBusy(true);
    setError(null);
    try {
      await create.mutateAsync({ ...parsed, case_id: caseId });
      onClose();
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  }

  return (
    <Modal title="Quick capture" onClose={onClose}>
      <form onSubmit={submit} className="space-y-4">
        <div>
          <Label>A lead, a link, a question</Label>
          <Textarea
            autoFocus
            rows={4}
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) submit();
            }}
            placeholder="Capture first. Connect later."
          />
        </div>
        <div>
          <Label>Suggested case file (optional)</Label>
          <select
            value={caseId ?? ""}
            onChange={(e) => setCaseId(e.target.value ? Number(e.target.value) : null)}
            className="w-full rounded-md border border-line bg-card px-3 py-2 text-sm outline-none focus:border-forest"
          >
            <option value="">Lead inbox (no case yet)</option>
            {cases?.map((c) => (
              <option key={c.id} value={c.id}>
                {c.code} · {c.title}
              </option>
            ))}
          </select>
        </div>
        {error && <p className="text-sm text-alert">{error}</p>}
        <div className="flex items-center justify-between pt-1">
          <span className="font-mono text-[11px] text-muted">⌘↵ to capture</span>
          <div className="flex gap-2">
            <Button type="button" variant="ghost" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" variant="primary" disabled={busy || !text.trim()}>
              Capture
            </Button>
          </div>
        </div>
      </form>
    </Modal>
  );
}
