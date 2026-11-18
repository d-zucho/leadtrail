import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/Button";
import { api, DOC_STATUSES, type Doc } from "@/lib/api";
import { useSetDocumentStatus } from "@/lib/queries";

const SELECT =
  "rounded-md border border-line bg-card px-2 py-1 text-xs outline-none focus:border-forest";

function fileName(title: string, ext: string) {
  const base = title.trim().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-+|-+$/g, "");
  return `${base || "article"}.${ext}`;
}

/** Status control and export actions for articles. */
export function ArticleBar({
  doc,
  title,
  flush,
}: {
  doc: Doc;
  title: string;
  /** Saves any pending edit so exports never miss the last keystrokes. */
  flush: () => Promise<void>;
}) {
  const setStatus = useSetDocumentStatus();
  const [error, setError] = useState<string | null>(null);

  async function run(fn: () => Promise<unknown>) {
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    }
  }

  const exportFile = (format: "markdown" | "html", ext: string) =>
    run(async () => {
      await flush();
      const path = await save({
        defaultPath: fileName(title, ext),
        filters: [{ name: format === "html" ? "HTML" : "Markdown", extensions: [ext] }],
      });
      if (path) await api.exportDocument(doc.id, format, path);
    });

  return (
    <div className="flex items-center gap-2">
      {error && (
        <span className="max-w-60 truncate text-xs text-alert" title={error}>
          {error}
        </span>
      )}
      <select
        aria-label="Publication status"
        value={doc.status}
        disabled={setStatus.isPending}
        onChange={(e) =>
          run(() => setStatus.mutateAsync({ id: doc.id, status: e.target.value as Doc["status"] }))
        }
        className={SELECT}
      >
        {DOC_STATUSES.map((s) => (
          <option key={s.value} value={s.value}>
            {s.label}
          </option>
        ))}
      </select>
      <Button className="px-3 py-1 text-xs" onClick={() => exportFile("markdown", "md")}>
        Markdown
      </Button>
      <Button className="px-3 py-1 text-xs" onClick={() => exportFile("html", "html")}>
        HTML
      </Button>
      <Button
        className="px-3 py-1 text-xs"
        title="Opens a print view in your browser; choose Save as PDF"
        onClick={() =>
          run(async () => {
            await flush();
            await api.openPrintView(doc.id);
          })
        }
      >
        PDF
      </Button>
    </div>
  );
}
