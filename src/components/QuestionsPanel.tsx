import { useState } from "react";
import { QuestionRow } from "@/components/QuestionRow";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Input } from "@/components/ui/Field";
import { useCreateQuestion, useQuestions } from "@/lib/queries";

export function QuestionsPanel({ caseId }: { caseId: number }) {
  const { data: questions } = useQuestions(caseId);
  const create = useCreateQuestion();
  const [text, setText] = useState("");
  const [due, setDue] = useState("");
  const [error, setError] = useState<string | null>(null);

  const open = (questions ?? []).filter((q) => !q.done).length;

  async function add(e: React.FormEvent) {
    e.preventDefault();
    if (!text.trim()) return;
    setError(null);
    try {
      await create.mutateAsync({ caseId, input: { text, due: due || undefined } });
      setText("");
      setDue("");
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <section className="mt-10">
      <h2 className="mb-3 font-display text-3xl">
        Open questions <span className="font-mono text-sm text-muted">{open}</span>
      </h2>
      <Card className="px-4 py-2">
        {questions && questions.length > 0 && (
          <div className="divide-y divide-line">
            {questions.map((q) => (
              <QuestionRow key={q.id} question={q} onError={setError} />
            ))}
          </div>
        )}
        <form
          onSubmit={add}
          className={`flex gap-2 py-3 ${questions && questions.length > 0 ? "border-t border-line" : ""}`}
        >
          <Input
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="What do you still need to find out?"
          />
          <input
            type="date"
            value={due}
            onChange={(e) => setDue(e.target.value)}
            aria-label="Due date"
            className="rounded-md border border-line bg-card px-2 py-2 text-sm outline-none focus:border-forest"
          />
          <Button type="submit" variant="primary" disabled={!text.trim() || create.isPending}>
            Add
          </Button>
        </form>
        {error && <p className="pb-2 text-sm text-alert">{error}</p>}
      </Card>
    </section>
  );
}
