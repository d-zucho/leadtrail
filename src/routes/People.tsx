import { useState } from "react";
import { Link, useSearchParams } from "react-router";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Input, Label, Textarea } from "@/components/ui/Field";
import { ENTITY_KINDS, type PartyDetail } from "@/lib/api";
import { useDeleteParty, useParty, usePartySearch, useUpdateParty } from "@/lib/queries";

const SELECT =
  "w-full rounded-md border border-line bg-card px-3 py-2 text-sm outline-none focus:border-forest";

const kindLabel = (kind: string) => ENTITY_KINDS.find((k) => k.value === kind)?.label ?? kind;

export function People() {
  const [params, setParams] = useSearchParams();
  const [query, setQuery] = useState("");
  const partyParam = Number(params.get("party"));
  const caseParam = Number(params.get("case"));
  const partyId = Number.isInteger(partyParam) && partyParam > 0 ? partyParam : undefined;
  const caseId = Number.isInteger(caseParam) && caseParam > 0 ? caseParam : undefined;

  const { data: rows, isLoading, error } = usePartySearch(query, caseId, 500);
  const { data: detail, error: detailError } = useParty(partyId, caseId);

  const select = (id: number | undefined) => {
    const next = new URLSearchParams(params);
    if (id === undefined) next.delete("party");
    else next.set("party", String(id));
    setParams(next);
  };

  return (
    <div>
      <h1 className="font-display text-5xl">People &amp; Companies</h1>
      <p className="mt-2 text-muted">
        Every person and organization you have recorded, across all case files. Names, types,
        aliases and descriptions are visible here for all records.
      </p>

      <div className="mt-6 grid grid-cols-[320px_1fr] gap-6">
        <div>
          <Input
            placeholder="Search by name or alias…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          {isLoading && <p className="mt-3 text-muted">Loading…</p>}
          {error && <p className="mt-3 text-alert">{String(error)}</p>}
          {rows && rows.length === 0 && (
            <p className="mt-3 text-sm text-muted">
              {query ? "Nothing matches." : "No people or companies yet."}
            </p>
          )}
          <ul className="mt-3 space-y-1">
            {(rows ?? []).map((r) => (
              <li key={r.id}>
                <button
                  onClick={() => select(r.id)}
                  className={`w-full rounded-md px-3 py-2 text-left ${
                    r.id === partyId ? "bg-forest text-white" : "hover:bg-sage"
                  }`}
                >
                  <div className="text-sm font-medium">{r.name}</div>
                  <div
                    className={`font-mono text-[10px] uppercase tracking-widest ${
                      r.id === partyId ? "text-white/70" : "text-muted"
                    }`}
                  >
                    {kindLabel(r.kind)}
                  </div>
                  {r.description && (
                    <div
                      className={`mt-0.5 line-clamp-2 text-xs ${
                        r.id === partyId ? "text-white/80" : "text-muted"
                      }`}
                    >
                      {r.description}
                    </div>
                  )}
                </button>
              </li>
            ))}
          </ul>
        </div>

        <div>
          {partyId === undefined ? (
            <Card className="p-8 text-center text-muted">
              Select a person or company to see its details.
            </Card>
          ) : detailError ? (
            <p className="text-alert">{String(detailError)}</p>
          ) : !detail ? (
            <p className="text-muted">Loading…</p>
          ) : (
            <PartyEditor
              key={`${detail.id}-${caseId ?? ""}`}
              detail={detail}
              onDeleted={() => select(undefined)}
            />
          )}
        </div>
      </div>
    </div>
  );
}

function PartyEditor({ detail, onDeleted }: { detail: PartyDetail; onDeleted: () => void }) {
  // Initialised once per record (the component is keyed), so background refetches
  // never overwrite in-progress edits.
  const [kind, setKind] = useState(detail.kind);
  const [name, setName] = useState(detail.name);
  const [aliases, setAliases] = useState(detail.aliases);
  const [description, setDescription] = useState(detail.description);
  const [error, setError] = useState<string | null>(null);
  const update = useUpdateParty();
  const del = useDeleteParty();

  const dirty =
    kind !== detail.kind ||
    name !== detail.name ||
    aliases !== detail.aliases ||
    description !== detail.description;

  const run = async (fn: () => Promise<unknown>) => {
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <Card className="space-y-6 p-6">
      <form
        className="space-y-3"
        onSubmit={(e) => {
          e.preventDefault();
          run(() =>
            update.mutateAsync({
              id: detail.id,
              input: { kind, name, aliases, description, shared: detail.shared },
            }),
          );
        }}
      >
        <div>
          <Label>Type</Label>
          <select value={kind} onChange={(e) => setKind(e.target.value)} className={SELECT}>
            {ENTITY_KINDS.map((k) => (
              <option key={k.value} value={k.value}>
                {k.label}
              </option>
            ))}
          </select>
        </div>
        <div>
          <Label>Name</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} />
        </div>
        <div>
          <Label>Aliases</Label>
          <Input value={aliases} onChange={(e) => setAliases(e.target.value)} />
        </div>
        <div>
          <Label>Description</Label>
          <Textarea rows={3} value={description} onChange={(e) => setDescription(e.target.value)} />
          <p className="mt-1.5 text-xs text-muted">
            Visible in every case file. Don't put sensitive case details here.
          </p>
        </div>
        <Button type="submit" variant="primary" disabled={!dirty || !name.trim() || update.isPending}>
          Save changes
        </Button>
      </form>

      <div className="space-y-3 border-t border-line pt-4">
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={detail.shared}
            disabled={update.isPending}
            onChange={(e) =>
              run(() =>
                update.mutateAsync({
                  id: detail.id,
                  input: {
                    kind: detail.kind,
                    name: detail.name,
                    aliases: detail.aliases,
                    description: detail.description,
                    shared: e.target.checked,
                  },
                }),
              )
            }
          />
          Show which cases this appears in
        </label>

        {detail.in_this_case && (
          <p className="text-sm">In this case: {detail.in_this_case.connections} connections</p>
        )}

        {detail.cases ? (
          detail.cases.length === 0 ? (
            <p className="text-sm text-muted">Not linked to any case.</p>
          ) : (
            <ul className="space-y-1">
              {detail.cases.map((c) => (
                <li key={c.case_id}>
                  <Link
                    to={`/cases/${c.case_id}`}
                    className="flex items-center gap-3 rounded-md px-2 py-1.5 text-sm hover:bg-sage"
                  >
                    <span className="font-mono text-xs text-muted">{c.code}</span>
                    <span>{c.title}</span>
                    <span className="ml-auto text-xs text-muted">
                      {c.connections} connections
                    </span>
                  </Link>
                </li>
              ))}
            </ul>
          )
        ) : (
          <p className="text-sm text-muted">
            Case details are hidden. Turn on “Show which cases this appears in” to list them.
          </p>
        )}
      </div>

      <div className="border-t border-line pt-4">
        <Button
          variant="danger"
          disabled={del.isPending}
          onClick={() =>
            run(async () => {
              await del.mutateAsync(detail.id);
              onDeleted();
            })
          }
        >
          Delete
        </Button>
      </div>

      {error && <p className="text-sm text-alert">{error}</p>}
    </Card>
  );
}
