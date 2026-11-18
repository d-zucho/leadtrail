import { Link } from "react-router";
import type { Question } from "@/lib/api";
import { formatDue, todayISO } from "@/lib/format";
import { useDeleteQuestion, useUpdateQuestion } from "@/lib/queries";

export function dueLabel(due: string | null, done: boolean) {
  if (!due) return { text: null as string | null, overdue: false };
  const today = todayISO();
  if (due === today) return { text: "Due today", overdue: !done };
  if (due < today) return { text: `Overdue · ${formatDue(due)}`, overdue: !done };
  return { text: `Due ${formatDue(due)}`, overdue: false };
}

export function QuestionRow({
  question: q,
  showCase = false,
  onError,
}: {
  question: Question;
  showCase?: boolean;
  onError?: (message: string) => void;
}) {
  const update = useUpdateQuestion();
  const del = useDeleteQuestion();
  const due = dueLabel(q.due, q.done);

  const fail = (e: unknown) => onError?.(String(e));

  return (
    <div className="group flex items-start gap-3 py-2.5">
      <input
        type="checkbox"
        checked={q.done}
        aria-label={q.done ? "Mark as open" : "Mark as answered"}
        onChange={() =>
          update
            .mutateAsync({
              id: q.id,
              input: { text: q.text, due: q.due ?? undefined, done: !q.done },
            })
            .catch(fail)
        }
        className="mt-1 h-4 w-4 shrink-0 accent-forest"
      />
      <div className="min-w-0 flex-1">
        <div className={q.done ? "text-muted line-through" : ""}>{q.text}</div>
        {(showCase || due.text) && (
          <div className="mt-0.5 font-mono text-[10px] uppercase tracking-wider">
            {showCase && (
              <Link to={`/cases/${q.case_id}`} className="text-muted hover:text-forest">
                {q.case_code}
              </Link>
            )}
            {showCase && due.text && <span className="text-muted"> · </span>}
            {due.text && (
              <span className={due.overdue ? "text-alert" : "text-muted"}>{due.text}</span>
            )}
          </div>
        )}
      </div>
      <button
        type="button"
        aria-label="Delete question"
        onClick={() => del.mutateAsync(q.id).catch(fail)}
        className="text-muted opacity-0 transition-opacity hover:text-alert focus:opacity-100 group-hover:opacity-100"
      >
        ×
      </button>
    </div>
  );
}
