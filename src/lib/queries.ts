import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  api,
  type CaseInput,
  type Doc,
  type DocKind,
  type DocStatus,
  type DocSave,
  type PartyInput,
  type LeadInput,
  type LeadUpdate,
  type QuestionInput,
  type QuestionUpdate,
  type RelationshipInput,
  type SourceInput,
  type SourceUpdate,
} from "./api";

// Sources affect case "updated" times, so refresh both.
const refreshSources = (qc: ReturnType<typeof useQueryClient>) => {
  qc.invalidateQueries({ queryKey: ["sources"] });
  qc.invalidateQueries({ queryKey: ["backlinks"] });
  qc.invalidateQueries({ queryKey: ["graph"] }); // deleting a source removes its evidence
  qc.invalidateQueries({ queryKey: ["cases"] });
};

export const useGraph = (caseId: number | undefined) =>
  useQuery({
    queryKey: ["graph", caseId],
    queryFn: () => api.getGraph(caseId!),
    enabled: caseId !== undefined,
  });

/** Any graph edit refreshes the graph and bumps the case's "updated" time. */
function useGraphMutation<V, R>(fn: (v: V) => Promise<R>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["graph"] });
      qc.invalidateQueries({ queryKey: ["cases"] });
      qc.invalidateQueries({ queryKey: ["parties"] });
    },
  });
}

export const useUpdateEntity = () =>
  useGraphMutation((v: { id: number; notes: string }) => api.updateEntity(v.id, v.notes));

export const usePartySearch = (query: string, caseId: number | undefined, limit?: number) =>
  useQuery({
    queryKey: ["parties", "search", query, caseId, limit],
    queryFn: () => api.searchParties(query, caseId, limit),
  });
export const useParty = (id: number | undefined, caseId: number | undefined) =>
  useQuery({
    queryKey: ["parties", "detail", id, caseId],
    queryFn: () => api.getParty(id!, caseId),
    enabled: id !== undefined,
  });
export const usePartyMentions = (partyId: number | undefined, caseId: number | undefined) =>
  useQuery({
    queryKey: ["parties", "mentions", partyId, caseId],
    queryFn: () => api.listPartyMentions(partyId!, caseId),
    enabled: partyId !== undefined,
  });

/** Party edits change names copied into involvements, so refresh parties, graph and cases. */
function usePartyMutation<V, R>(fn: (v: V) => Promise<R>) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["parties"] });
      qc.invalidateQueries({ queryKey: ["graph"] });
      qc.invalidateQueries({ queryKey: ["cases"] });
    },
  });
}
export const useLinkParty = () =>
  usePartyMutation((v: { caseId: number; partyId: number }) => api.linkParty(v.caseId, v.partyId));
export const useCreatePartyAndLink = () =>
  usePartyMutation((v: { caseId: number; input: PartyInput }) =>
    api.createPartyAndLink(v.caseId, v.input),
  );
export const useCreateParty = () =>
  usePartyMutation((input: PartyInput) => api.createParty(input));
export const useUpdateParty = () =>
  usePartyMutation((v: { id: number; input: PartyInput }) => api.updateParty(v.id, v.input));
export const useDeleteParty = () => usePartyMutation((id: number) => api.deleteParty(id));
export const useDeleteEntity = () => useGraphMutation((id: number) => api.deleteEntity(id));
export const useCreateRelationship = () =>
  useGraphMutation((v: { caseId: number; input: RelationshipInput }) =>
    api.createRelationship(v.caseId, v.input),
  );
export const useUpdateRelationship = () =>
  useGraphMutation((v: { id: number; input: { label: string; notes?: string } }) =>
    api.updateRelationship(v.id, v.input),
  );
export const useDeleteRelationship = () =>
  useGraphMutation((id: number) => api.deleteRelationship(id));
export const useSetEvidence = () =>
  useGraphMutation((v: { relationshipId: number; sourceId: number; quote?: string; locator?: string }) =>
    api.setEvidence(v.relationshipId, v.sourceId, v.quote, v.locator),
  );
export const useRemoveEvidence = () =>
  useGraphMutation((v: { relationshipId: number; sourceId: number }) =>
    api.removeEvidence(v.relationshipId, v.sourceId),
  );

export const useQuestions =(caseId?: number) =>
  useQuery({
    queryKey: ["questions", caseId ?? "all"],
    queryFn: () => api.listQuestions(caseId),
  });

export const useActivity = (limit = 8) =>
  useQuery({ queryKey: ["activity", limit], queryFn: () => api.recentActivity(limit) });

// Question changes alter the case cards' open-question counts too.
const refreshQuestions = (qc: ReturnType<typeof useQueryClient>) => {
  qc.invalidateQueries({ queryKey: ["questions"] });
  qc.invalidateQueries({ queryKey: ["cases"] });
};

export function useCreateQuestion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { caseId: number; input: QuestionInput }) =>
      api.createQuestion(v.caseId, v.input),
    onSuccess: () => refreshQuestions(qc),
  });
}

export function useUpdateQuestion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { id: number; input: QuestionUpdate }) => api.updateQuestion(v.id, v.input),
    onSuccess: () => refreshQuestions(qc),
  });
}

export function useDeleteQuestion() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteQuestion(id),
    onSuccess: () => refreshQuestions(qc),
  });
}

export const useDocuments =(caseId: number) =>
  useQuery({ queryKey: ["documents", "case", caseId], queryFn: () => api.listDocuments(caseId) });

/**
 * The editor owns the live content, so this must not refetch behind its back.
 * Saves write their result into this cache entry instead.
 */
export const useDocument = (id: number) =>
  useQuery({
    queryKey: ["documents", "one", id],
    queryFn: () => api.getDocument(id),
    staleTime: Infinity,
    refetchOnWindowFocus: false,
  });

export const useBacklinks = (id: number) =>
  useQuery({ queryKey: ["backlinks", "document", id], queryFn: () => api.listBacklinks(id) });

export const useSourceCitations = (id: number) =>
  useQuery({ queryKey: ["backlinks", "source", id], queryFn: () => api.listSourceCitations(id) });

export function useCreateDocument() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { caseId: number; kind: DocKind; title?: string }) =>
      api.createDocument(v.caseId, v.kind, v.title),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["documents", "case"] });
      qc.invalidateQueries({ queryKey: ["queue"] });
      qc.invalidateQueries({ queryKey: ["cases"] });
    },
  });
}

export function useSaveDocument() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { id: number; input: DocSave }) => api.saveDocument(v.id, v.input),
    onSuccess: (doc) => {
      qc.setQueryData(["documents", "one", doc.id], doc);
      qc.invalidateQueries({ queryKey: ["documents", "case"] });
      qc.invalidateQueries({ queryKey: ["queue"] });
      qc.invalidateQueries({ queryKey: ["backlinks"] });
      qc.invalidateQueries({ queryKey: ["parties"] });
    },
  });
}

export const useQueue = () => useQuery({ queryKey: ["queue"], queryFn: api.listQueue });

export function useSetDocumentStatus() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { id: number; status: DocStatus }) => api.setDocumentStatus(v.id, v.status),
    onSuccess: (doc) => {
      // Only the status fields change; leave the editor's content untouched.
      qc.setQueryData<Doc>(["documents", "one", doc.id], (old) =>
        old ? { ...old, status: doc.status, published_at: doc.published_at } : old,
      );
      qc.invalidateQueries({ queryKey: ["documents", "case"] });
      qc.invalidateQueries({ queryKey: ["queue"] });
    },
  });
}

export function useDeleteDocument() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteDocument(id),
    onSuccess: (_d, id) => {
      qc.removeQueries({ queryKey: ["documents", "one", id] });
      qc.invalidateQueries({ queryKey: ["documents", "case"] });
      qc.invalidateQueries({ queryKey: ["queue"] });
      qc.invalidateQueries({ queryKey: ["backlinks"] });
      qc.invalidateQueries({ queryKey: ["cases"] });
    },
  });
}

export const useLeads =() => useQuery({ queryKey: ["leads"], queryFn: api.listLeads });

export function useCreateLead() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: LeadInput) => api.createLead(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["leads"] }),
  });
}

export function useUpdateLead() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { id: number; input: LeadUpdate }) => api.updateLead(v.id, v.input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["leads"] }),
  });
}

export function usePromoteLead() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { id: number; caseId: number }) => api.promoteLead(v.id, v.caseId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["leads"] });
      refreshSources(qc);
    },
  });
}

export function useDeleteLead() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteLead(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["leads"] }),
  });
}

export const useSources =(caseId?: number) =>
  useQuery({ queryKey: ["sources", caseId ?? "all"], queryFn: () => api.listSources(caseId) });

export function useCreateSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: SourceInput) => api.createSource(input),
    onSuccess: () => refreshSources(qc),
  });
}

export function useImportFile() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { caseId: number; path: string; title?: string }) =>
      api.importFile(v.caseId, v.path, v.title),
    onSuccess: () => refreshSources(qc),
  });
}

export function useUpdateSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { id: number; input: SourceUpdate }) => api.updateSource(v.id, v.input),
    onSuccess: () => refreshSources(qc),
  });
}

export function useDeleteSource() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteSource(id),
    onSuccess: () => refreshSources(qc),
  });
}

export const useCases = () =>
  useQuery({ queryKey: ["cases"], queryFn: api.listCases });

export const useCase = (id: number) =>
  useQuery({ queryKey: ["cases", id], queryFn: () => api.getCase(id) });

export function useCreateCase() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CaseInput) => api.createCase(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["cases"] }),
  });
}

export function useUpdateCase(id: number) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CaseInput) => api.updateCase(id, input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["cases"] }),
  });
}

export function useDeleteCase() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteCase(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["cases"] });
      qc.invalidateQueries({ queryKey: ["sources"] });
      qc.invalidateQueries({ queryKey: ["leads"] });
      qc.invalidateQueries({ queryKey: ["documents"] });
      qc.invalidateQueries({ queryKey: ["queue"] });
      qc.invalidateQueries({ queryKey: ["backlinks"] });
      qc.invalidateQueries({ queryKey: ["questions"] });
      qc.invalidateQueries({ queryKey: ["graph"] });
      qc.invalidateQueries({ queryKey: ["parties"] });
    },
  });
}
