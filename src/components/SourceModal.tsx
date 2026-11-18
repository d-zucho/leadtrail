import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input, Label, Textarea } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import { api, VERIFICATIONS, type Source, type Verification } from "@/lib/api";
import { formatBytes } from "@/lib/format";
import { useDeleteSource, useUpdateSource } from "@/lib/queries";

export function SourceModal({ source, onClose }: { source: Source; onClose: () => void }) {
  const [title, setTitle] = useState(source.title);
  const [url, setUrl] = useState(source.url ?? "");
  const [notes, setNotes] = useState(source.notes);
  const [status, setStatus] = useState<Verification>(source.verification_status);
  const [vnote, setVnote] = useState(source.verification_note);
  const [error, setError] = useState<string | null>(null);
  const update = useUpdateSource();
  const del = useDeleteSource();

  async function save(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await update.mutateAsync({
        id: source.id,
        input: {
          title,
          url,
          notes,
          verification_status: status,
          verification_note: vnote,
        },
      });
      onClose();
    } catch (err) {
      setError(String(err));
    }
  }

  async function remove() {
    const msg =
      source.kind === "file"
        ? `Delete "${source.title}" and its stored file? This cannot be undone.`
        : `Delete "${source.title}"? This cannot be undone.`;
    if (!window.confirm(msg)) return;
    try {
      await del.mutateAsync(source.id);
      onClose();
    } catch (err) {
      setError(String(err));
    }
  }

  async function openFile() {
    setError(null);
    try {
      await api.openSourceFile(source.id);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <Modal title="Source" onClose={onClose}>
      <form onSubmit={save} className="space-y-4">
        <div>
          <Label>Title</Label>
          <Input value={title} onChange={(e) => setTitle(e.target.value)} />
        </div>

        {source.kind !== "file" && source.kind !== "note" && (
          <div>
            <Label>URL</Label>
            <Input value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://" />
          </div>
        )}

        {source.kind === "file" && (
          <div className="rounded-md border border-line bg-card p-3 text-xs">
            <div className="flex items-center justify-between">
              <span>
                {source.file_name}
                {source.size != null && (
                  <span className="text-muted"> · {formatBytes(source.size)}</span>
                )}
              </span>
              <Button type="button" onClick={openFile} className="px-3 py-1 text-xs">
                Open file
              </Button>
            </div>
            <div className="mt-2 break-all font-mono text-[10px] text-muted">
              SHA-256 {source.hash}
            </div>
          </div>
        )}

        <div>
          <Label>Notes</Label>
          <Textarea rows={3} value={notes} onChange={(e) => setNotes(e.target.value)} />
        </div>

        <div className="grid grid-cols-[10rem_1fr] gap-3">
          <div>
            <Label>Verification</Label>
            <select
              value={status}
              onChange={(e) => setStatus(e.target.value as Verification)}
              className="w-full rounded-md border border-line bg-card px-3 py-2 text-sm outline-none focus:border-forest"
            >
              {VERIFICATIONS.map((v) => (
                <option key={v.value} value={v.value}>
                  {v.label}
                </option>
              ))}
            </select>
          </div>
          <div>
            <Label>Verification note</Label>
            <Input
              value={vnote}
              onChange={(e) => setVnote(e.target.value)}
              placeholder="What was checked, and how?"
            />
          </div>
        </div>

        {error && <p className="text-sm text-alert">{error}</p>}

        <div className="flex items-center justify-between pt-2">
          <Button type="button" variant="danger" onClick={remove}>
            Delete
          </Button>
          <div className="flex gap-2">
            <Button type="button" variant="ghost" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" variant="primary" disabled={!title.trim()}>
              Save
            </Button>
          </div>
        </div>
      </form>
    </Modal>
  );
}
