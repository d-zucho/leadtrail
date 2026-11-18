import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/Button";
import { Input, Label, Textarea } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import { TEXT_SOURCE_KINDS, type SourceInput } from "@/lib/api";
import { useCreateSource, useImportFile } from "@/lib/queries";

type Mode = "file" | SourceInput["kind"];

export function AddSourceModal({ caseId, onClose }: { caseId: number; onClose: () => void }) {
  const [mode, setMode] = useState<Mode>("file");
  const [title, setTitle] = useState("");
  const [url, setUrl] = useState("");
  const [notes, setNotes] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const importFile = useImportFile();
  const createSource = useCreateSource();

  async function pickFiles() {
    setErrors([]);
    const picked = await open({ multiple: true, directory: false });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    setBusy(true);
    const failed: string[] = [];
    for (const path of paths) {
      try {
        await importFile.mutateAsync({ caseId, path });
      } catch (e) {
        const name = path.split(/[\\/]/).pop();
        failed.push(`${name}: ${String(e)}`);
      }
    }
    setBusy(false);
    if (failed.length === 0) onClose();
    else setErrors(failed);
  }

  async function submitText(e: React.FormEvent) {
    e.preventDefault();
    if (mode === "file") return;
    setBusy(true);
    setErrors([]);
    try {
      await createSource.mutateAsync({ case_id: caseId, kind: mode, title, url, notes });
      onClose();
    } catch (err) {
      setErrors([String(err)]);
      setBusy(false);
    }
  }

  const tabs: { value: Mode; label: string }[] = [
    { value: "file", label: "File" },
    ...TEXT_SOURCE_KINDS,
  ];

  return (
    <Modal title="Add a source" onClose={onClose}>
      <div className="mb-5 flex flex-wrap gap-1">
        {tabs.map((t) => (
          <button
            key={t.value}
            type="button"
            onClick={() => {
              setMode(t.value);
              setErrors([]);
            }}
            className={`rounded-md px-3 py-1.5 text-sm ${
              mode === t.value ? "bg-forest text-white" : "text-muted hover:bg-sage"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {mode === "file" ? (
        <div className="space-y-4">
          <p className="text-sm text-muted">
            Files are copied into your vault and fingerprinted with SHA-256, so you can show later
            that nothing has changed. The original stays where it is.
          </p>
          <Button variant="primary" onClick={pickFiles} disabled={busy}>
            {busy ? "Importing…" : "Choose files…"}
          </Button>
        </div>
      ) : (
        <form onSubmit={submitText} className="space-y-4">
          <div>
            <Label>Title</Label>
            <Input autoFocus value={title} onChange={(e) => setTitle(e.target.value)} />
          </div>
          {(mode === "link" || mode === "record") && (
            <div>
              <Label>URL</Label>
              <Input
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://"
              />
            </div>
          )}
          <div>
            <Label>Notes</Label>
            <Textarea rows={4} value={notes} onChange={(e) => setNotes(e.target.value)} />
          </div>
          <div className="flex justify-end">
            <Button type="submit" variant="primary" disabled={busy || !title.trim()}>
              Add source
            </Button>
          </div>
        </form>
      )}

      {errors.length > 0 && (
        <ul className="mt-4 space-y-1 text-sm text-alert">
          {errors.map((e) => (
            <li key={e}>{e}</li>
          ))}
        </ul>
      )}
    </Modal>
  );
}
