import { invoke } from "@tauri-apps/api/core";

export type CaseStatus =
  | "investigating"
  | "verification"
  | "drafting"
  | "published"
  | "archived";

export const CASE_STATUSES: CaseStatus[] = [
  "investigating",
  "verification",
  "drafting",
  "published",
  "archived",
];

export interface Case {
  id: number;
  code: string;
  title: string;
  summary: string;
  status: CaseStatus;
  created_at: string;
  updated_at: string;
  source_count: number;
  document_count: number;
  open_questions: number;
}

export interface Question {
  id: number;
  case_id: number;
  case_code: string;
  text: string;
  due: string | null;
  done: boolean;
  created_at: string;
}

export interface QuestionInput {
  text: string;
  due?: string;
}

export interface QuestionUpdate {
  text: string;
  due?: string;
  done: boolean;
}

export interface Activity {
  kind: "case" | "source" | "document" | "lead";
  title: string;
  case_id: number | null;
  case_code: string | null;
  doc_id: number | null;
  at: string;
}

export interface CaseInput {
  title: string;
  summary?: string;
  status?: CaseStatus;
}

export type SourceKind = "file" | "link" | "note" | "interview" | "record";
export type Verification = "unverified" | "to_review" | "verified" | "disputed";

export const VERIFICATIONS: { value: Verification; label: string }[] = [
  { value: "unverified", label: "Unverified" },
  { value: "to_review", label: "To review" },
  { value: "verified", label: "Verified" },
  { value: "disputed", label: "Disputed" },
];

export const TEXT_SOURCE_KINDS: { value: Exclude<SourceKind, "file">; label: string }[] = [
  { value: "link", label: "Link" },
  { value: "note", label: "Note" },
  { value: "interview", label: "Interview" },
  { value: "record", label: "Public record" },
];

export interface Source {
  id: number;
  case_id: number;
  case_code: string;
  case_title: string;
  kind: SourceKind;
  title: string;
  url: string | null;
  notes: string;
  file_name: string | null;
  size: number | null;
  hash: string | null;
  verification_status: Verification;
  verification_note: string;
  created_at: string;
  updated_at: string;
}

export interface SourceInput {
  case_id: number;
  kind: Exclude<SourceKind, "file">;
  title: string;
  url?: string;
  notes?: string;
}

export interface SourceUpdate {
  title: string;
  url?: string;
  notes?: string;
  verification_status: Verification;
  verification_note?: string;
}

export type LeadKind = "note" | "link" | "tip" | "document";
export type LeadStatus = "new" | "to_review" | "promoted" | "dismissed";

export interface Lead {
  id: number;
  title: string;
  kind: LeadKind;
  url: string | null;
  body: string;
  case_id: number | null;
  case_code: string | null;
  status: LeadStatus;
  created_at: string;
  updated_at: string;
}

export interface LeadInput {
  title: string;
  kind?: LeadKind;
  url?: string;
  body?: string;
  case_id?: number | null;
}

export interface LeadUpdate {
  title: string;
  url?: string;
  body?: string;
  case_id?: number | null;
  status: LeadStatus;
}

export type DocKind = "note" | "article";

export type DocStatus = "draft" | "in_review" | "ready" | "published";

export const DOC_STATUSES: { value: DocStatus; label: string }[] = [
  { value: "draft", label: "Draft" },
  { value: "in_review", label: "In review" },
  { value: "ready", label: "Ready" },
  { value: "published", label: "Published" },
];

export interface QueueItem {
  id: number;
  case_id: number;
  case_code: string;
  case_title: string;
  title: string;
  status: DocStatus;
  words: number;
  published_at: string | null;
  updated_at: string;
}

export interface Doc {
  id: number;
  case_id: number;
  kind: DocKind;
  title: string;
  content_json: string;
  status: DocStatus;
  published_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface DocSummary {
  id: number;
  case_id: number;
  kind: DocKind;
  title: string;
  excerpt: string;
  status: DocStatus;
  updated_at: string;
}

export interface Backlink {
  doc_id: number;
  case_id: number;
  kind: DocKind;
  title: string;
}

export interface LinkRef {
  dst_type: "document" | "source";
  dst_id: number;
  label: string;
}

export interface DocSave {
  title: string;
  content_json: string;
  content_text: string;
  links: LinkRef[];
}

export type EntityKind = "person" | "organization" | "public_body";

export const ENTITY_KINDS: { value: EntityKind; label: string }[] = [
  { value: "person", label: "Person" },
  { value: "organization", label: "Organization" },
  { value: "public_body", label: "Public body" },
];

export interface Entity {
  id: number;
  case_id: number;
  kind: EntityKind;
  name: string;
  notes: string;
  party_id: number;
  x: number | null;
  y: number | null;
}

export interface Evidence {
  source_id: number;
  source_title: string;
  source_kind: SourceKind;
  verification_status: Verification;
  quote: string;
  locator: string;
}

export interface Relationship {
  id: number;
  case_id: number;
  from_id: number;
  to_id: number;
  label: string;
  notes: string;
  evidence: Evidence[];
}

export interface Graph {
  entities: Entity[];
  relationships: Relationship[];
}

export interface PartyInput {
  kind: string;
  name: string;
  aliases?: string;
  description?: string;
  shared?: boolean;
}

export interface PartyRow {
  id: number;
  kind: string;
  name: string;
  aliases: string;
  description: string;
  shared: boolean;
  in_this_case: boolean;
  other_case_count: number | null;
}

export interface CaseRef {
  case_id: number;
  code: string;
  title: string;
  connections: number;
}

export interface PartyDetail {
  id: number;
  kind: string;
  name: string;
  aliases: string;
  description: string;
  shared: boolean;
  in_this_case: CaseRef | null;
  /** Non-null only when the record is shared. */
  cases: CaseRef[] | null;
}

export interface RelationshipInput {
  from_id: number;
  to_id: number;
  label: string;
  notes?: string;
}

export type HitKind = "case" | "source" | "lead" | "document" | "question" | "entity" | "party";

export interface Hit {
  kind: HitKind;
  ref_id: number;
  case_id: number | null;
  case_code: string | null;
  title: string;
  /** Match text; matches are wrapped in \u0001 … \u0002. */
  snippet: string;
}

export interface DataInfo {
  path: string;
  evidence_files: number;
  evidence_bytes: number;
  db_bytes: number;
}

export const api = {
  dataInfo: () => invoke<DataInfo>("data_info"),
  revealDataDir: () => invoke<void>("reveal_data_dir"),
  backupVault: (path: string) => invoke<{ files: number; bytes: number }>("backup_vault", { path }),
  restoreBackup: (path: string) => invoke<void>("restore_backup", { path }),
  search: (query: string) => invoke<Hit[]>("search", { query }),

  getGraph: (caseId: number) => invoke<Graph>("get_graph", { caseId }),
  updateEntity: (id: number, notes: string) => invoke<void>("update_entity", { id, notes }),
  searchParties: (query: string, caseId?: number, limit?: number) =>
    invoke<PartyRow[]>("search_parties", { query, caseId, limit }),
  getParty: (id: number, caseId?: number) => invoke<PartyDetail>("get_party", { id, caseId }),
  createParty: (input: PartyInput) => invoke<number>("create_party", { input }),
  updateParty: (id: number, input: PartyInput) => invoke<void>("update_party", { id, input }),
  deleteParty: (id: number) => invoke<void>("delete_party", { id }),
  linkParty: (caseId: number, partyId: number) =>
    invoke<number>("link_party", { caseId, partyId }),
  createPartyAndLink: (caseId: number, input: PartyInput) =>
    invoke<number>("create_party_and_link", { caseId, input }),
  moveEntity: (id: number, x: number, y: number) => invoke<void>("move_entity", { id, x, y }),
  deleteEntity: (id: number) => invoke<void>("delete_entity", { id }),
  createRelationship: (caseId: number, input: RelationshipInput) =>
    invoke<number>("create_relationship", { caseId, input }),
  updateRelationship: (id: number, input: { label: string; notes?: string }) =>
    invoke<void>("update_relationship", { id, input }),
  deleteRelationship: (id: number) => invoke<void>("delete_relationship", { id }),
  setEvidence: (relationshipId: number, sourceId: number, quote?: string, locator?: string) =>
    invoke<void>("set_evidence", {
      relationshipId,
      sourceId,
      quote: quote ?? null,
      locator: locator ?? null,
    }),
  removeEvidence: (relationshipId: number, sourceId: number) =>
    invoke<void>("remove_evidence", { relationshipId, sourceId }),
  listQuestions: (caseId?: number) =>
    invoke<Question[]>("list_questions", { caseId: caseId ?? null }),
  createQuestion: (caseId: number, input: QuestionInput) =>
    invoke<Question>("create_question", { caseId, input }),
  updateQuestion: (id: number, input: QuestionUpdate) =>
    invoke<Question>("update_question", { id, input }),
  deleteQuestion: (id: number) => invoke<void>("delete_question", { id }),
  recentActivity: (limit?: number) =>
    invoke<Activity[]>("recent_activity", { limit: limit ?? null }),
  listDocuments: (caseId: number) => invoke<DocSummary[]>("list_documents", { caseId }),
  getDocument: (id: number) => invoke<Doc>("get_document", { id }),
  createDocument: (caseId: number, kind: DocKind, title?: string) =>
    invoke<Doc>("create_document", { caseId, kind, title: title ?? null }),
  saveDocument: (id: number, input: DocSave) => invoke<Doc>("save_document", { id, input }),
  setDocumentStatus: (id: number, status: DocStatus) =>
    invoke<Doc>("set_document_status", { id, status }),
  listQueue: () => invoke<QueueItem[]>("list_queue"),
  exportDocument: (id: number, format: "markdown" | "html", path: string) =>
    invoke<void>("export_document", { id, format, path }),
  openPrintView: (id: number) => invoke<void>("open_print_view", { id }),
  deleteDocument: (id: number) => invoke<void>("delete_document", { id }),
  listBacklinks: (id: number) => invoke<Backlink[]>("list_backlinks", { id }),
  listSourceCitations: (id: number) => invoke<Backlink[]>("list_source_citations", { id }),
  listLeads: () => invoke<Lead[]>("list_leads"),
  createLead: (input: LeadInput) => invoke<Lead>("create_lead", { input }),
  updateLead: (id: number, input: LeadUpdate) => invoke<Lead>("update_lead", { id, input }),
  promoteLead: (id: number, caseId: number) =>
    invoke<Lead>("promote_lead", { id, caseId }),
  deleteLead: (id: number) => invoke<void>("delete_lead", { id }),
  listSources: (caseId?: number) =>
    invoke<Source[]>("list_sources", { caseId: caseId ?? null }),
  createSource: (input: SourceInput) => invoke<Source>("create_source", { input }),
  importFile: (caseId: number, path: string, title?: string) =>
    invoke<Source>("import_file", { caseId, path, title: title ?? null }),
  updateSource: (id: number, input: SourceUpdate) =>
    invoke<Source>("update_source", { id, input }),
  deleteSource: (id: number) => invoke<void>("delete_source", { id }),
  openSourceFile: (id: number) => invoke<void>("open_source_file", { id }),

  listCases: () => invoke<Case[]>("list_cases"),
  getCase: (id: number) => invoke<Case>("get_case", { id }),
  createCase: (input: CaseInput) => invoke<Case>("create_case", { input }),
  updateCase: (id: number, input: CaseInput) =>
    invoke<Case>("update_case", { id, input }),
  deleteCase: (id: number) => invoke<void>("delete_case", { id }),
};
