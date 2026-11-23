import { useEffect, useState } from "react";
import { Link } from "react-router";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Input, Label, Textarea } from "@/components/ui/Field";
import { VerificationBadge } from "@/components/ui/VerificationBadge";
import {
  ENTITY_KINDS,
  type Entity,
  type Graph,
  type Relationship,
  type Source,
} from "@/lib/api";
import {
  useDeleteEntity,
  useDeleteRelationship,
  useParty,
  useRemoveEvidence,
  useSetEvidence,
  useUpdateEntity,
  useUpdateParty,
  useUpdateRelationship,
} from "@/lib/queries";

const SELECT =
  "w-full rounded-md border border-line bg-card px-3 py-2 text-sm outline-none focus:border-forest";

const SMALL = "px-3 py-1 text-xs";

/** Runs an async action and routes any failure into the panel's error line. */
function useAction() {
  const [error, setError] = useState<string | null>(null);
  const run = async (fn: () => Promise<unknown>) => {
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    }
  };
  return { error, run };
}

function PanelTitle({ children }: { children: React.ReactNode }) {
  return (
    <div className="font-mono text-[10px] uppercase tracking-widest text-muted">{children}</div>
  );
}

export function EntityPanel({
  caseId,
  entity,
  graph,
  onSelectRelationship,
  onConnect,
  onDeleted,
}: {
  caseId: number;
  entity: Entity;
  graph: Graph;
  onSelectRelationship: (id: number) => void;
  onConnect: () => void;
  onDeleted: () => void;
}) {
  const { data: party } = useParty(entity.party_id, caseId);
  const [kind, setKind] = useState<string>(entity.kind);
  const [name, setName] = useState(entity.name);
  const [aliases, setAliases] = useState("");
  const [description, setDescription] = useState("");
  const [notes, setNotes] = useState(entity.notes);
  const [loaded, setLoaded] = useState(false);
  const updateParty = useUpdateParty();
  const updateEntity = useUpdateEntity();
  const del = useDeleteEntity();
  const { error, run } = useAction();

  // Fill identity fields once from the party record (the panel is keyed by entity).
  useEffect(() => {
    if (party && !loaded) {
      setKind(party.kind);
      setName(party.name);
      setAliases(party.aliases);
      setDescription(party.description);
      setLoaded(true);
    }
  }, [party, loaded]);

  const byId = new Map(graph.entities.map((e) => [e.id, e]));
  const touching = graph.relationships.filter(
    (r) => r.from_id === entity.id || r.to_id === entity.id,
  );
  const identityDirty =
    !!party &&
    (kind !== party.kind ||
      name !== party.name ||
      aliases !== party.aliases ||
      description !== party.description);
  const notesDirty = notes !== entity.notes;
  const pending = updateParty.isPending || updateEntity.isPending;

  return (
    <div className="space-y-5">
      <PanelTitle>Entity</PanelTitle>
      <form
        className="space-y-3"
        onSubmit={(e) => {
          e.preventDefault();
          run(async () => {
            if (identityDirty && party) {
              await updateParty.mutateAsync({
                id: party.id,
                input: { kind, name, aliases, description, shared: party.shared },
              });
            }
            if (notesDirty) await updateEntity.mutateAsync({ id: entity.id, notes });
          });
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
          <Textarea rows={2} value={description} onChange={(e) => setDescription(e.target.value)} />
          <p className="mt-1.5 text-xs text-muted">
            Visible in every case file. Don't put sensitive case details here.
          </p>
        </div>
        <p className="text-xs text-muted">
          Changes to type, name, aliases and description apply in every case file that uses this
          record.
        </p>
        <div>
          <Label>Notes</Label>
          <Textarea rows={3} value={notes} onChange={(e) => setNotes(e.target.value)} />
        </div>
        <div className="flex items-center justify-between gap-2">
          <Button
            type="submit"
            variant="primary"
            className={SMALL}
            disabled={!loaded || (!identityDirty && !notesDirty) || !name.trim() || pending}
          >
            Save changes
          </Button>
          <Link
            to={`/people?party=${entity.party_id}&case=${caseId}`}
            className="text-xs text-forest hover:underline"
          >
            Open in People &amp; Companies
          </Link>
        </div>
      </form>

      <div>
        <div className="mb-2 flex items-center justify-between">
          <PanelTitle>Connections · {touching.length}</PanelTitle>
          <Button className={SMALL} onClick={onConnect} disabled={graph.entities.length < 2}>
            + Connect
          </Button>
        </div>
        {touching.length === 0 ? (
          <p className="text-sm text-muted">
            {graph.entities.length < 2
              ? "Add another entity to connect this one to."
              : "Not connected to anything yet."}
          </p>
        ) : (
          <ul className="space-y-1">
            {touching.map((r) => {
              const outgoing = r.from_id === entity.id;
              const other = byId.get(outgoing ? r.to_id : r.from_id);
              return (
                <li key={r.id}>
                  <button
                    onClick={() => onSelectRelationship(r.id)}
                    className="w-full rounded-md px-2 py-1.5 text-left text-sm hover:bg-sage"
                  >
                    <span className="text-muted">{outgoing ? "→" : "←"} </span>
                    <span className="font-mono text-[11px] uppercase text-muted">{r.label}</span>{" "}
                    {other?.name}
                    <span className="ml-1 text-xs text-muted">
                      {r.evidence.length === 0 ? "· no evidence" : `· ${r.evidence.length} cited`}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      {error && <p className="text-sm text-alert">{error}</p>}

      <Button
        variant="danger"
        className={SMALL}
        onClick={() => {
          const msg = touching.length
            ? `Delete "${entity.name}" and its ${touching.length} connection${touching.length === 1 ? "" : "s"}?`
            : `Delete "${entity.name}"?`;
          if (window.confirm(msg)) {
            run(async () => {
              await del.mutateAsync(entity.id);
              onDeleted();
            });
          }
        }}
      >
        Delete entity
      </Button>
    </div>
  );
}

export function RelationshipPanel({
  rel,
  graph,
  sources,
  onSelectEntity,
  onOpenSource,
  onDeleted,
}: {
  rel: Relationship;
  graph: Graph;
  sources: Source[];
  onSelectEntity: (id: number) => void;
  onOpenSource: (id: number) => void;
  onDeleted: () => void;
}) {
  const [label, setLabel] = useState(rel.label);
  const [notes, setNotes] = useState(rel.notes);
  const [sourceId, setSourceId] = useState<number | "">("");
  const [quote, setQuote] = useState("");
  const [locator, setLocator] = useState("");
  const update = useUpdateRelationship();
  const del = useDeleteRelationship();
  const setEvidence = useSetEvidence();
  const removeEvidence = useRemoveEvidence();
  const { error, run } = useAction();

  const from = graph.entities.find((e) => e.id === rel.from_id);
  const to = graph.entities.find((e) => e.id === rel.to_id);
  const dirty = label !== rel.label || notes !== rel.notes;

  return (
    <div className="space-y-5">
      <PanelTitle>Relationship</PanelTitle>
      <div className="text-sm leading-relaxed">
        <button onClick={() => from && onSelectEntity(from.id)} className="font-medium hover:text-forest">
          {from?.name}
        </button>
        <span className="mx-1.5 font-mono text-[11px] uppercase text-muted">{rel.label} →</span>
        <button onClick={() => to && onSelectEntity(to.id)} className="font-medium hover:text-forest">
          {to?.name}
        </button>
      </div>

      <form
        className="space-y-3"
        onSubmit={(e) => {
          e.preventDefault();
          run(() => update.mutateAsync({ id: rel.id, input: { label, notes } }));
        }}
      >
        <div>
          <Label>Relationship</Label>
          <Input value={label} onChange={(e) => setLabel(e.target.value)} />
        </div>
        <div>
          <Label>Notes</Label>
          <Textarea rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} />
        </div>
        <Button
          type="submit"
          variant="primary"
          className={SMALL}
          disabled={!dirty || !label.trim() || update.isPending}
        >
          Save changes
        </Button>
      </form>

      <div>
        <PanelTitle>Linked evidence · {rel.evidence.length}</PanelTitle>
        {rel.evidence.length === 0 && (
          <p className="mt-2 rounded-md bg-sand px-3 py-2 text-sm text-[#7a5d1c]">
            Nothing backs this connection yet. Until a source is linked, treat it as a claim, not
            a fact.
          </p>
        )}
        <div className="mt-2 space-y-3">
          {rel.evidence.map((ev) => (
            <Card key={ev.source_id} className="p-3">
              {ev.quote && (
                <blockquote className="font-display text-lg leading-snug">“{ev.quote}”</blockquote>
              )}
              <div className={`flex items-center justify-between gap-2 ${ev.quote ? "mt-2" : ""}`}>
                <button
                  onClick={() => onOpenSource(ev.source_id)}
                  className="min-w-0 truncate text-left text-sm hover:text-forest"
                >
                  {ev.source_title}
                  {ev.locator && <span className="text-muted"> · {ev.locator}</span>}
                </button>
                <VerificationBadge status={ev.verification_status} />
              </div>
              <button
                onClick={() =>
                  run(() =>
                    removeEvidence.mutateAsync({ relationshipId: rel.id, sourceId: ev.source_id }),
                  )
                }
                className="mt-2 font-mono text-[10px] uppercase tracking-wider text-muted hover:text-alert"
              >
                Remove
              </button>
            </Card>
          ))}
        </div>

        <form
          className="mt-3 space-y-2 rounded-lg border border-dashed border-line p-3"
          onSubmit={(e) => {
            e.preventDefault();
            if (sourceId === "") return;
            run(async () => {
              await setEvidence.mutateAsync({
                relationshipId: rel.id,
                sourceId,
                quote,
                locator,
              });
              setSourceId("");
              setQuote("");
              setLocator("");
            });
          }}
        >
          <Label>Link a source</Label>
          <select
            value={sourceId}
            onChange={(e) => setSourceId(e.target.value ? Number(e.target.value) : "")}
            className={SELECT}
          >
            <option value="">
              {sources.length === 0 ? "No sources in this case yet" : "Choose a source…"}
            </option>
            {sources.map((s) => (
              <option key={s.id} value={s.id}>
                {s.title}
              </option>
            ))}
          </select>
          <Textarea
            rows={2}
            value={quote}
            onChange={(e) => setQuote(e.target.value)}
            placeholder="Quote or passage that supports it"
          />
          <Input
            value={locator}
            onChange={(e) => setLocator(e.target.value)}
            placeholder="Where, e.g. p. 4"
          />
          <Button
            type="submit"
            className={SMALL}
            disabled={sourceId === "" || setEvidence.isPending}
          >
            Add evidence
          </Button>
          <p className="text-xs text-muted">
            Linking a source you already used replaces its quote.
          </p>
        </form>
      </div>

      {error && <p className="text-sm text-alert">{error}</p>}

      <Button
        variant="danger"
        className={SMALL}
        onClick={() => {
          if (window.confirm("Delete this connection and its evidence links?")) {
            run(async () => {
              await del.mutateAsync(rel.id);
              onDeleted();
            });
          }
        }}
      >
        Delete connection
      </Button>
    </div>
  );
}
