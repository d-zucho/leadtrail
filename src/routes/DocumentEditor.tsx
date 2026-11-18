import { ArticleBar } from "@/components/ArticleBar";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";
import { EditorContent, useEditor } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import { SourceModal } from "@/components/SourceModal";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { buildExtensions, type EditorContext, type LinkTarget } from "@/editor/extensions";
import { extractLinks, parseTargetId } from "@/editor/links";
import type { Doc } from "@/lib/api";
import {
  useBacklinks,
  useCreateDocument,
  useDeleteDocument,
  useDocument,
  useDocuments,
  useSaveDocument,
  useSources,
} from "@/lib/queries";

type SaveState = "saved" | "dirty" | "saving" | "error";

const SAVE_LABEL: Record<SaveState, string> = {
  saved: "All changes saved",
  dirty: "Unsaved changes",
  saving: "Saving…",
  error: "Could not save",
};

export function DocumentEditor() {
  const params = useParams();
  const caseId = Number(params.id);
  const docId = Number(params.docId);
  const { data: doc, isLoading, error } = useDocument(docId);

  if (isLoading) return <p className="text-muted">Loading…</p>;
  if (error || !doc) return <p className="text-alert">Document not found.</p>;
  // Keyed by id so switching documents always starts a fresh editor and save queue.
  return <EditorView key={doc.id} doc={doc} caseId={caseId} />;
}

function parseContent(json: string): JSONContent | undefined {
  if (!json) return undefined;
  try {
    return JSON.parse(json) as JSONContent;
  } catch {
    return undefined;
  }
}

function EditorView({ doc, caseId }: { doc: Doc; caseId: number }) {
  const navigate = useNavigate();
  const save = useSaveDocument();
  const del = useDeleteDocument();
  const createDoc = useCreateDocument();
  const { data: docs } = useDocuments(caseId);
  const { data: sources } = useSources(caseId);
  const { data: backlinks } = useBacklinks(doc.id);

  const [title, setTitle] = useState(doc.title);
  const [status, setStatus] = useState<SaveState>("saved");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [openSourceId, setOpenSourceId] = useState<number | null>(null);

  // Latest values live in refs so the debounced/unmount save never reads stale state.
  const latest = useRef<{ title: string; json: JSONContent | null; text: string }>({
    title: doc.title,
    json: null,
    text: "",
  });
  const pending = useRef(false);
  const inFlight = useRef(false);
  const timer = useRef<number | undefined>(undefined);
  // The mutation object changes identity every render; keep flush stable via a ref.
  const saveRef = useRef(save.mutateAsync);
  saveRef.current = save.mutateAsync;
  const flushRef = useRef<() => Promise<void>>(async () => {});

  const clearTimer = useCallback(() => {
    window.clearTimeout(timer.current);
    timer.current = undefined;
  }, []);

  const flush = useCallback(async () => {
    if (!pending.current || inFlight.current) return;
    pending.current = false;
    inFlight.current = true;
    setStatus("saving");
    const { title: t, json, text } = latest.current;
    let failed = false;
    try {
      await saveRef.current({
        id: doc.id,
        input: {
          title: t,
          content_json: JSON.stringify(json ?? { type: "doc", content: [] }),
          content_text: text,
          links: extractLinks(json ?? { type: "doc" }, doc.id),
        },
      });
      setSaveError(null);
      setStatus(pending.current ? "dirty" : "saved");
    } catch (e) {
      failed = true;
      pending.current = true; // stay dirty so the next edit retries
      setSaveError(String(e));
      setStatus("error");
    } finally {
      inFlight.current = false;
    }
    // An edit made while this save was running was skipped by the in-flight guard.
    if (!failed && pending.current && timer.current === undefined) {
      timer.current = window.setTimeout(() => {
        timer.current = undefined;
        void flushRef.current();
      }, 300);
    }
  }, [doc.id]);
  flushRef.current = flush;

  const schedule = useCallback(() => {
    pending.current = true;
    setStatus("dirty");
    clearTimer();
    timer.current = window.setTimeout(() => {
      timer.current = undefined;
      void flushRef.current();
    }, 800);
  }, [clearTimer]);

  // Targets for [[ ]] — everything in this case except the document being edited.
  const targets = useMemo<LinkTarget[]>(
    () => [
      ...(docs ?? [])
        .filter((d) => d.id !== doc.id)
        .map((d) => ({
          id: `doc:${d.id}`,
          label: d.title || "Untitled",
          hint: d.kind,
        })),
      ...(sources ?? []).map((s) => ({ id: `src:${s.id}`, label: s.title, hint: "source" })),
    ],
    [docs, sources, doc.id],
  );
  const targetsRef = useRef(targets);
  targetsRef.current = targets;

  const createDocRef = useRef(createDoc.mutateAsync);
  createDocRef.current = createDoc.mutateAsync;

  const extensions = useMemo(() => {
    const ctx: EditorContext = {
      getTargets: () => targetsRef.current,
      createNote: async (t) => (await createDocRef.current({ caseId, kind: "note", title: t })).id,
    };
    return buildExtensions(ctx);
  }, [caseId]);

  const editor = useEditor({
    extensions,
    content: parseContent(doc.content_json),
    editorProps: { attributes: { class: "prose-editor" } },
    onCreate: ({ editor }) => {
      latest.current.json = editor.getJSON();
      latest.current.text = editor.getText();
    },
    onUpdate: ({ editor }) => {
      latest.current.json = editor.getJSON();
      latest.current.text = editor.getText();
      schedule();
    },
  });

  // Save on unmount and when the window loses focus, so closing never drops edits.
  useEffect(() => {
    const onBlur = () => {
      clearTimer();
      void flushRef.current();
    };
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("blur", onBlur);
      clearTimer();
      void flushRef.current();
    };
  }, [clearTimer]);

  function onTitleChange(v: string) {
    setTitle(v);
    latest.current.title = v;
    schedule();
  }

  function onContentClick(e: React.MouseEvent) {
    const el = (e.target as HTMLElement).closest('[data-type="mention"]');
    const target = parseTargetId(el?.getAttribute("data-id"));
    if (!target) return;
    e.preventDefault();
    // Make sure the current edit is saved before leaving this document.
    if (target.type === "document") {
      clearTimer();
      void flush().then(() => navigate(`/cases/${caseId}/docs/${target.id}`));
    } else {
      setOpenSourceId(target.id);
    }
  }

  async function remove() {
    if (!window.confirm(`Delete "${title || "Untitled"}"? This cannot be undone.`)) return;
    clearTimer();
    pending.current = false; // nothing to save for a document that is going away
    await del.mutateAsync(doc.id);
    navigate(`/cases/${caseId}`);
  }

  const openSource = sources?.find((s) => s.id === openSourceId);

  return (
    <div className="mx-auto flex max-w-6xl gap-10">
      <div className="min-w-0 flex-1">
        <div className="flex items-center justify-between">
          <Link to={`/cases/${caseId}`} className="font-mono text-xs text-muted hover:text-ink">
            ← Case file
          </Link>
          <div className="flex items-center gap-3">
            <span
              className={`font-mono text-[11px] ${status === "error" ? "text-alert" : "text-muted"}`}
              title={saveError ?? undefined}
            >
              {SAVE_LABEL[status]}
            </span>
            {doc.kind === "article" && <ArticleBar doc={doc} title={title} flush={flush} />}
            <Button variant="ghost" onClick={remove} className="px-3 py-1 text-xs">
              Delete
            </Button>
          </div>
        </div>

        <div className="mt-6 font-mono text-[10px] uppercase tracking-widest text-muted">
          {doc.kind}
        </div>
        <input
          value={title}
          onChange={(e) => onTitleChange(e.target.value)}
          placeholder="Untitled"
          className="mt-1 w-full bg-transparent font-display text-5xl leading-tight outline-none placeholder:text-line"
        />

        <div className="mt-6" onClick={onContentClick}>
          <EditorContent editor={editor} />
        </div>
      </div>

      <aside className="hidden w-64 shrink-0 pt-10 lg:block">
        <Card className="p-4">
          <div className="font-mono text-[10px] uppercase tracking-widest text-muted">
            Linked from
          </div>
          {backlinks && backlinks.length > 0 ? (
            <ul className="mt-3 space-y-2">
              {backlinks.map((b) => (
                <li key={b.doc_id}>
                  <Link
                    to={`/cases/${b.case_id}/docs/${b.doc_id}`}
                    className="text-sm hover:text-forest"
                  >
                    {b.title || "Untitled"}
                  </Link>
                  <div className="font-mono text-[10px] uppercase text-muted">{b.kind}</div>
                </li>
              ))}
            </ul>
          ) : (
            <p className="mt-3 text-sm text-muted">
              Nothing links here yet. Type [[ in another note to link to this one.
            </p>
          )}
        </Card>
      </aside>

      {openSource && <SourceModal source={openSource} onClose={() => setOpenSourceId(null)} />}
    </div>
  );
}
