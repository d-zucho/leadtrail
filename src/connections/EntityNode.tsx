import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import type { Entity, EntityKind } from "@/lib/api";

export type EntityNodeData = { entity: Entity };
export type EntityFlowNode = Node<EntityNodeData, "entity">;

export const KIND_LABEL: Record<EntityKind, string> = {
  person: "PERSON",
  organization: "ORGANIZATION",
  public_body: "PUBLIC BODY",
};

const KIND_TINT: Record<EntityKind, string> = {
  person: "bg-sand",
  organization: "bg-card",
  public_body: "bg-sage",
};

export function EntityNode({ data, selected }: NodeProps<EntityFlowNode>) {
  const { entity } = data;
  return (
    <div
      className={`min-w-40 max-w-56 rounded-lg border px-3 py-2 shadow-sm ${KIND_TINT[entity.kind]} ${
        selected ? "border-forest ring-2 ring-forest/30" : "border-line"
      }`}
    >
      <Handle type="target" position={Position.Top} className="!bg-forest" />
      <div className="font-mono text-[9px] tracking-widest text-muted">
        {KIND_LABEL[entity.kind]}
      </div>
      <div className="mt-0.5 text-sm font-medium leading-snug">{entity.name}</div>
      <Handle type="source" position={Position.Bottom} className="!bg-forest" />
    </div>
  );
}
