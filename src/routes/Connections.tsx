import { useEffect, useMemo, useRef, useState } from "react";
import { Link, useSearchParams } from "react-router";
import {
  Background,
  Controls,
  MarkerType,
  ReactFlow,
  type Connection,
  type Edge,
  type OnNodesChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useQueryClient } from "@tanstack/react-query";
import { SourceModal } from "@/components/SourceModal";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import {
  EntityNode,
  type EntityFlowNode,
} from "@/connections/EntityNode";
import { ConnectModal, EntityModal } from "@/connections/Modals";
import { EntityPanel, RelationshipPanel } from "@/connections/Panels";
import { api, type Entity, type Graph, type Relationship } from "@/lib/api";
import { useCases, useGraph, useSources } from "@/lib/queries";

const nodeTypes = { entity: EntityNode };

type Selection = { kind: "entity" | "edge"; id: number } | null;

const COLOR = {
  verified: "#2f5d4c",
  unverified: "#b08a2e",
  none: "#9aa097",
};

function circle(i: number, n: number) {
  const r = Math.max(170, n * 40);
  const a = (2 * Math.PI * i) / Math.max(n, 1) - Math.PI / 2;
  return { x: Math.round(Math.cos(a) * r), y: Math.round(Math.sin(a) * r) };
}

/**
 * Builds the node list from saved entities, keeping any position already on screen.
 * Entities with no saved position get one, and are returned so it can be saved too,
 * which keeps the layout stable across reloads.
 */
function layoutNodes(entities: Entity[], prev: EntityFlowNode[], selectedId: number | null) {
  const prevPos = new Map(prev.map((n) => [n.id, n.position]));
  const maxX = prev.length ? Math.max(...prev.map((n) => n.position.x)) : 0;
  const minY = prev.length ? Math.min(...prev.map((n) => n.position.y)) : 0;
  const toPersist: { id: number; x: number; y: number }[] = [];
  let added = 0;

  const nodes: EntityFlowNode[] = entities.map((e, i) => {
    const key = String(e.id);
    let position = prevPos.get(key);
    if (!position && e.x != null && e.y != null) position = { x: e.x, y: e.y };
    if (!position) {
      // New arrival next to an existing layout goes in a free column to the right.
      position = prev.length
        ? { x: maxX + 220, y: minY + added++ * 90 }
        : circle(i, entities.length);
      toPersist.push({ id: e.id, ...position });
    }
    return {
      id: key,
      type: "entity",
      position,
      data: { entity: e },
      selected: e.id === selectedId,
    };
  });
  return { nodes, toPersist };
}

function edgeColor(r: Relationship) {
  if (r.evidence.length === 0) return COLOR.none;
  return r.evidence.some((e) => e.verification_status === "verified")
    ? COLOR.verified
    : COLOR.unverified;
}

export function Connections() {
  const [params, setParams] = useSearchParams();
  const { data: cases, isLoading } = useCases();

  const requested = Number(params.get("case"));
  const caseId = cases?.some((c) => c.id === requested) ? requested : cases?.[0]?.id;

  return (
    <div>
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="font-display text-5xl">Connections</h1>
          <p className="mt-2 text-muted">
            Who is tied to whom, and what proves it. Drag from a node's bottom dot to another's top
            dot to connect them.
          </p>
        </div>
        {cases && cases.length > 0 && caseId !== undefined && (
          <select
            value={caseId}
            onChange={(e) => setParams({ case: e.target.value })}
            aria-label="Case file"
            className="rounded-md border border-line bg-card px-3 py-2 text-sm outline-none focus:border-forest"
          >
            {cases.map((c) => (
              <option key={c.id} value={c.id}>
                {c.code} · {c.title}
              </option>
            ))}
          </select>
        )}
      </div>

      <div className="mt-6">
        {isLoading && <p className="text-muted">Loading…</p>}
        {cases && cases.length === 0 && (
          <Card className="px-8 py-14 text-center">
            <h2 className="font-display text-3xl">No case files yet</h2>
            <p className="mx-auto mt-2 max-w-sm text-muted">
              Connections belong to a case file. Create one first.
            </p>
            <Link to="/cases">
              <Button variant="primary" className="mt-6">
                Go to case files
              </Button>
            </Link>
          </Card>
        )}
        {/* Keyed by case so selection, modals, and layout reset when switching. */}
        {caseId !== undefined && <Workspace key={caseId} caseId={caseId} />}
      </div>
    </div>
  );
}

function Workspace({ caseId }: { caseId: number }) {
  const qc = useQueryClient();
  const { data: graph, isLoading, error } = useGraph(caseId);
  const { data: sources } = useSources(caseId);

  const [selection, setSelection] = useState<Selection>(null);
  const [adding, setAdding] = useState(false);
  const [connecting, setConnecting] = useState<{ from: number; to?: number } | null>(null);
  const [openSourceId, setOpenSourceId] = useState<number | null>(null);
  const [nodes, setNodes] = useState<EntityFlowNode[]>([]);

  const nodesRef = useRef(nodes);
  nodesRef.current = nodes;
  const persisted = useRef(new Set<number>());

  /** Saves a position and mirrors it into the cache so a stale copy can never win later. */
  function savePosition(id: number, x: number, y: number) {
    api
      .moveEntity(id, x, y)
      .then(() =>
        qc.setQueryData<Graph>(["graph", caseId], (g) =>
          g
            ? { ...g, entities: g.entities.map((e) => (e.id === id ? { ...e, x, y } : e)) }
            : g,
        ),
      )
      .catch((e) => console.error("could not save position", e));
  }

  const selectedEntityId = selection?.kind === "entity" ? selection.id : null;

  useEffect(() => {
    if (!graph) return;
    const { nodes: next, toPersist } = layoutNodes(
      graph.entities,
      nodesRef.current,
      selectedEntityId,
    );
    setNodes(next);
    for (const p of toPersist) {
      if (persisted.current.has(p.id)) continue;
      persisted.current.add(p.id);
      savePosition(p.id, p.x, p.y);
    }
    // savePosition only closes over stable values (qc, caseId)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graph, selectedEntityId]);

  const edges = useMemo<Edge[]>(() => {
    if (!graph) return [];
    return graph.relationships.map((r) => {
      const color = edgeColor(r);
      const selected = selection?.kind === "edge" && selection.id === r.id;
      return {
        id: String(r.id),
        source: String(r.from_id),
        target: String(r.to_id),
        label: r.evidence.length ? `${r.label} · ${r.evidence.length}` : r.label,
        selected,
        markerEnd: { type: MarkerType.ArrowClosed, color },
        style: {
          stroke: color,
          strokeWidth: selected ? 3 : 1.75,
          strokeDasharray: r.evidence.length === 0 ? "6 4" : undefined,
        },
        labelStyle: { fontFamily: "JetBrains Mono, monospace", fontSize: 10, fill: "#4a524c" },
        labelBgStyle: { fill: "#f6f4ee" },
        labelBgPadding: [6, 3],
      } satisfies Edge;
    });
  }, [graph, selection]);

  const entity =
    selection?.kind === "entity" ? graph?.entities.find((e) => e.id === selection.id) : undefined;
  const rel =
    selection?.kind === "edge" ? graph?.relationships.find((r) => r.id === selection.id) : undefined;
  const openSource = sources?.find((s) => s.id === openSourceId);

  const onNodesChange: OnNodesChange<EntityFlowNode> = (changes) =>
    setNodes((ns) => changes.applyTo(ns));

  const onConnect = (c: Connection) =>
    setConnecting({ from: Number(c.source), to: Number(c.target) });

  if (isLoading) return <p className="text-muted">Loading…</p>;
  if (error || !graph) return <p className="text-alert">Could not load connections.</p>;

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-[1fr_22rem]">
      <div className="relative h-[calc(100vh-16rem)] min-h-[30rem] overflow-hidden rounded-lg border border-line bg-card">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodesChange={onNodesChange}
          onConnect={onConnect}
          onNodeClick={(_, n) => setSelection({ kind: "entity", id: Number(n.id) })}
          onEdgeClick={(_, e) => setSelection({ kind: "edge", id: Number(e.id) })}
          onPaneClick={() => setSelection(null)}
          onNodeDragStop={(_, n) => savePosition(Number(n.id), n.position.x, n.position.y)}
          // Deleting must go through the panel so the database stays the source of truth.
          deleteKeyCode={null}
          fitView
          fitViewOptions={{ padding: 0.3, maxZoom: 1 }}
          minZoom={0.3}
        >
          <Background gap={24} color="#e3dfd3" />
          <Controls showInteractive={false} />
        </ReactFlow>

        <div className="absolute left-3 top-3 z-10">
          <Button variant="primary" onClick={() => setAdding(true)} className="py-1.5 text-xs">
            + Add entity
          </Button>
        </div>

        <Legend />

        {graph.entities.length === 0 && (
          <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
            <div className="max-w-xs text-center">
              <div className="font-display text-3xl">Nothing mapped yet</div>
              <p className="mt-2 text-sm text-muted">
                Add the people, organizations, and public bodies in this case, then connect them.
              </p>
            </div>
          </div>
        )}
      </div>

      <Card className="h-fit max-h-[calc(100vh-16rem)] overflow-y-auto p-5">
        {entity ? (
          <EntityPanel
            key={entity.id}
            entity={entity}
            graph={graph}
            onSelectRelationship={(id) => setSelection({ kind: "edge", id })}
            onConnect={() => setConnecting({ from: entity.id })}
            onDeleted={() => setSelection(null)}
          />
        ) : rel ? (
          <RelationshipPanel
            key={rel.id}
            rel={rel}
            graph={graph}
            sources={sources ?? []}
            onSelectEntity={(id) => setSelection({ kind: "entity", id })}
            onOpenSource={setOpenSourceId}
            onDeleted={() => setSelection(null)}
          />
        ) : (
          <div>
            <div className="font-mono text-[10px] uppercase tracking-widest text-muted">
              Overview
            </div>
            <p className="mt-3 text-sm text-muted">
              Select an entity to edit it or connect it. Select a connection to see, and add, the
              evidence behind it.
            </p>
            <dl className="mt-4 grid grid-cols-2 gap-3 text-sm">
              <Stat label="Entities" value={graph.entities.length} />
              <Stat label="Connections" value={graph.relationships.length} />
              <Stat
                label="Unsupported"
                value={graph.relationships.filter((r) => r.evidence.length === 0).length}
              />
              <Stat
                label="Verified"
                value={
                  graph.relationships.filter((r) =>
                    r.evidence.some((e) => e.verification_status === "verified"),
                  ).length
                }
              />
            </dl>
          </div>
        )}
      </Card>

      {adding && (
        <EntityModal
          caseId={caseId}
          onClose={() => setAdding(false)}
          onCreated={(id) => setSelection({ kind: "entity", id })}
        />
      )}
      {connecting && (
        <ConnectModal
          caseId={caseId}
          entities={graph.entities}
          fromId={connecting.from}
          toId={connecting.to}
          onClose={() => setConnecting(null)}
          onCreated={(id) => setSelection({ kind: "edge", id })}
        />
      )}
      {openSource && <SourceModal source={openSource} onClose={() => setOpenSourceId(null)} />}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div>
      <dt className="font-mono text-[10px] uppercase tracking-widest text-muted">{label}</dt>
      <dd className="font-display text-3xl leading-none">{String(value).padStart(2, "0")}</dd>
    </div>
  );
}

function Legend() {
  const items = [
    { color: COLOR.verified, label: "Verified source", dashed: false },
    { color: COLOR.unverified, label: "Unverified source", dashed: false },
    { color: COLOR.none, label: "No evidence", dashed: true },
  ];
  return (
    <div className="pointer-events-none absolute right-3 top-3 z-10 space-y-1 rounded-md bg-paper/90 px-3 py-2">
      {items.map((i) => (
        <div key={i.label} className="flex items-center gap-2 text-[11px] text-muted">
          <svg width="22" height="4" aria-hidden>
            <line
              x1="0"
              y1="2"
              x2="22"
              y2="2"
              stroke={i.color}
              strokeWidth="2"
              strokeDasharray={i.dashed ? "4 3" : undefined}
            />
          </svg>
          {i.label}
        </div>
      ))}
    </div>
  );
}
