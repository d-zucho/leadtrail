import { useDeferredValue, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input, Label, Textarea } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import { PartyPicker } from "@/connections/PartyPicker";
import { ENTITY_KINDS, type Entity, type EntityKind, type PartyRow } from "@/lib/api";
import {
  useCreatePartyAndLink,
  useCreateRelationship,
  useLinkParty,
  usePartySearch,
  useUpdateEntity,
} from "@/lib/queries";

const SELECT =
  "w-full rounded-md border border-line bg-card px-3 py-2 text-sm outline-none focus:border-forest";

/** Option label for an entity: name plus description, or `#id` when it would be ambiguous. */
export function entityLabel(e: Entity, all: Entity[]): string {
  if (e.description) return `${e.name} — ${e.description}`;
  const same = (o: Entity) => o.id !== e.id && o.name.trim().toLowerCase() === e.name.trim().toLowerCase();
  const dup = all.some(same);
  return dup ? `${e.name} (#${e.party_id})` : e.name;
}

export function EntityModal({
  caseId,
  onClose,
  onCreated,
}: {
  caseId: number;
  onClose: () => void;
  onCreated: (id: number) => void;
}) {
  const [kind, setKind] = useState<EntityKind>("organization");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);
  const link = useLinkParty();
  const create = useCreatePartyAndLink();
  const updateEntity = useUpdateEntity();

  async function pick(party: PartyRow) {
    setError(null);
    try {
      const id = await link.mutateAsync({ caseId, partyId: party.id });
      if (notes.trim()) await updateEntity.mutateAsync({ id, notes });
      onCreated(id);
      onClose();
    } catch (err) {
      setError(String(err));
    }
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      const id = await create.mutateAsync({ caseId, input: { kind, name, description } });
      if (notes.trim()) await updateEntity.mutateAsync({ id, notes });
      onCreated(id);
      onClose();
    } catch (err) {
      setError(String(err));
    }
  }

  const deferredName = useDeferredValue(name);
  const { data: matches } = usePartySearch(deferredName, caseId, 8);
  const typed = name.trim().toLowerCase();
  const hasSameName = typed !== "" && (matches ?? []).some((p) => p.name.trim().toLowerCase() === typed);

  const busy = link.isPending || create.isPending || updateEntity.isPending;

  return (
    <Modal title="Add an entity" onClose={onClose}>
      <form onSubmit={submit} className="space-y-4">
        <div>
          <Label>Type</Label>
          <select
            value={kind}
            onChange={(e) => setKind(e.target.value as EntityKind)}
            className={SELECT}
          >
            {ENTITY_KINDS.map((k) => (
              <option key={k.value} value={k.value}>
                {k.label}
              </option>
            ))}
          </select>
        </div>
        <div>
          <Label>Name</Label>
          <Input autoFocus value={name} onChange={(e) => setName(e.target.value)} />
          <PartyPicker caseId={caseId} query={name} onPick={pick} />
        </div>
        <div>
          <Label>Description</Label>
          <Textarea rows={2} value={description} onChange={(e) => setDescription(e.target.value)} />
          <p className="mt-1.5 text-xs text-muted">
            Visible in every case file. Don't put sensitive case details here.
          </p>
        </div>
        <div>
          <Label>Notes</Label>
          <Textarea rows={3} value={notes} onChange={(e) => setNotes(e.target.value)} />
        </div>
        {error && <p className="text-sm text-alert">{error}</p>}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" variant="primary" disabled={!name.trim() || busy}>
            {hasSameName ? `Create another “${name.trim()}”` : "Create new"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

const LABEL_SUGGESTIONS = [
  "director of",
  "owns",
  "employs",
  "awarded contract to",
  "paid",
  "married to",
  "related to",
];

export function ConnectModal({
  caseId,
  entities,
  fromId,
  toId,
  onClose,
  onCreated,
}: {
  caseId: number;
  entities: Entity[];
  fromId: number;
  toId?: number;
  onClose: () => void;
  onCreated: (relationshipId: number) => void;
}) {
  const [from, setFrom] = useState(fromId);
  const [to, setTo] = useState<number | "">(toId ?? "");
  const [label, setLabel] = useState("");
  const [error, setError] = useState<string | null>(null);
  const create = useCreateRelationship();

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (to === "") return;
    setError(null);
    try {
      const id = await create.mutateAsync({
        caseId,
        input: { from_id: from, to_id: to, label },
      });
      onCreated(id);
      onClose();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <Modal title="Connect entities" onClose={onClose}>
      <form onSubmit={submit} className="space-y-4">
        <div className="grid grid-cols-2 gap-3">
          <div>
            <Label>From</Label>
            <select value={from} onChange={(e) => setFrom(Number(e.target.value))} className={SELECT}>
              {entities.map((e) => (
                <option key={e.id} value={e.id}>
                  {entityLabel(e, entities)}
                </option>
              ))}
            </select>
          </div>
          <div>
            <Label>To</Label>
            <select
              value={to}
              onChange={(e) => setTo(e.target.value ? Number(e.target.value) : "")}
              className={SELECT}
            >
              <option value="">Choose…</option>
              {entities
                .filter((e) => e.id !== from)
                .map((e) => (
                  <option key={e.id} value={e.id}>
                    {entityLabel(e, entities)}
                  </option>
                ))}
            </select>
          </div>
        </div>
        <div>
          <Label>Relationship</Label>
          <Input
            autoFocus
            list="relationship-labels"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="e.g. director of"
          />
          <datalist id="relationship-labels">
            {LABEL_SUGGESTIONS.map((s) => (
              <option key={s} value={s} />
            ))}
          </datalist>
          <p className="mt-1.5 text-xs text-muted">
            Reads as: “From <em>{label.trim() || "…"}</em> To”. You can attach evidence afterwards.
          </p>
        </div>
        {error && <p className="text-sm text-alert">{error}</p>}
        <div className="flex justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="primary"
            disabled={to === "" || !label.trim() || create.isPending}
          >
            Connect
          </Button>
        </div>
      </form>
    </Modal>
  );
}
