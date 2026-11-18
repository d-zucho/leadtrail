import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useQuery } from "@tanstack/react-query";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { api } from "@/lib/api";
import { formatBytes, todayISO } from "@/lib/format";

export function Settings() {
  const { data: info } = useQuery({ queryKey: ["data-info"], queryFn: api.dataInfo });
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  async function run(fn: () => Promise<string | void>) {
    setBusy(true);
    setMessage(null);
    try {
      const text = await fn();
      if (text) setMessage({ ok: true, text });
    } catch (e) {
      setMessage({ ok: false, text: String(e) });
    } finally {
      setBusy(false);
    }
  }

  const backup = () =>
    run(async () => {
      const path = await save({
        defaultPath: `fieldnotes-backup-${todayISO()}.zip`,
        filters: [{ name: "Fieldnotes backup", extensions: ["zip"] }],
      });
      if (!path) return;
      const r = await api.backupVault(path);
      return `Backed up the database and ${r.files} evidence file${r.files === 1 ? "" : "s"} (${formatBytes(r.bytes)}).`;
    });

  const restore = () =>
    run(async () => {
      const path = await open({
        multiple: false,
        filters: [{ name: "Fieldnotes backup", extensions: ["zip"] }],
      });
      if (typeof path !== "string") return;
      const ok = window.confirm(
        "Restore this backup? Fieldnotes will restart and replace everything now in the app. " +
          "Your current data is kept in a pre-restore folder.",
      );
      if (ok) await api.restoreBackup(path); // restarts the app on success
    });

  return (
    <div className="max-w-2xl">
      <h1 className="font-display text-5xl">Settings</h1>

      <section className="mt-8">
        <h2 className="font-display text-3xl">Your data</h2>
        <Card className="mt-3 space-y-2 p-4 text-sm">
          <p className="text-muted">
            Everything lives on this computer. Fieldnotes makes no network requests.
          </p>
          {info && (
            <>
              <div className="break-all font-mono text-xs">{info.path}</div>
              <div className="text-muted">
                Database {formatBytes(info.db_bytes)} · {info.evidence_files} evidence file
                {info.evidence_files === 1 ? "" : "s"} ({formatBytes(info.evidence_bytes)})
              </div>
            </>
          )}
          <Button className="px-3 py-1 text-xs" onClick={() => run(api.revealDataDir)}>
            Show in file manager
          </Button>
        </Card>
      </section>

      <section className="mt-8">
        <h2 className="font-display text-3xl">Backup &amp; restore</h2>
        <Card className="mt-3 space-y-3 p-4 text-sm">
          <p className="text-muted">
            A backup is one zip with your database and every evidence file. It is not encrypted, so
            store it somewhere you trust.
          </p>
          <div className="flex gap-2">
            <Button variant="primary" onClick={backup} disabled={busy}>
              Back up…
            </Button>
            <Button onClick={restore} disabled={busy}>
              Restore from backup…
            </Button>
          </div>
          {message && (
            <p className={message.ok ? "text-forest" : "text-alert"}>{message.text}</p>
          )}
        </Card>
      </section>
    </div>
  );
}
