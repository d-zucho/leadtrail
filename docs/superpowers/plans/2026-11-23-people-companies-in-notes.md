# People and Companies Inside Notes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Typing `[[` in a note can mention a person or company; their record page lists the notes that mention them, respecting the existing sharing switch.

**Architecture:** A mention is a TipTap `mention` node with id `party:<id>`, stored in the existing `links` table with `dst_type = 'party'` (no migration). A new `mentions.rs` resolves chip labels to current record names inside `documents::fetch` (so opening and exporting a note always show current names) and lists a record's mentions with the visibility rule enforced in Rust. The `[[` picker gains record rows through the existing `search_parties`; the People page gains a "Mentioned in" section.

**Tech Stack:** Rust (rusqlite, serde_json), Tauri 2 commands, React 19, TipTap 3, TanStack Query, Tailwind v4.

**Spec:** `docs/superpowers/specs/2026-11-23-people-companies-in-notes-design.md`

## Global Constraints

- No new migration (schema stays at version 9). If one proves unavoidable it is migration 10, appended; never edit an existing `MIGRATIONS` entry.
- Every new `#[tauri::command]` must be registered in `invoke_handler` in `src-tauri/src/lib.rs`. Logic lives in plain functions taking `&Connection`; commands are thin wrappers.
- Validate in Rust. The mention-visibility rule is enforced in Rust, and error messages never name another case.
- Chip ids are `party:<id>`; existing ids stay `doc:<id>` and `src:<id>`.
- A party target does **not** need to be in the same case as the note (records are global). Document and source targets keep the same-case rule.
- A note save must never fail because a mentioned record is missing or deleted.
- Exports stay publish-safe: a party chip exports as the record's current name as plain text; no `party:` text may appear in Markdown or HTML output.
- Mentions do not add anyone to the graph and do not block deleting a record. Deleting a record deletes its `party` rows in `links`.
- Visibility on the record page: notes in the given `case_id` are always listed; notes in other cases only when the record is shared; with no `case_id` and an unshared record nothing is listed.
- Frontend: `@/` alias, Tailwind tokens from `src/index.css`, no network calls, no new dependencies. There is no frontend test runner: frontend tasks are verified with `pnpm build` and the manual checks listed.
- Commit steps run only if the user has asked for commits in this session; otherwise leave changes uncommitted and say so. Commit messages end with the attribution line the session specifies.

## Review Focus

1. A record's mentions from another case's notes are never returned for an unshared record (including the serialized JSON), and `case_code` appears only for notes outside the given `case_id`.
2. Saving a note that mentions a missing or since-deleted record succeeds and silently drops that link.
3. Deleting a record removes its `party` links, leaves the notes intact, and clicking a leftover chip for it shows a readable "not found" on the People page instead of crashing.
4. After a record is renamed, reopening a note and exporting it (Markdown and HTML) show the new name and contain no `party:` text.
5. Existing `[[` flows are unchanged: note and source links, the create-note row, a note's own backlinks (party rows must not appear there), and a `[[` query with punctuation or quotes does not break the party search.
6. Fast typing after `[[` never leaves stale party results from an earlier keystroke in the menu.

---

### Task 1: Backend: party links, live labels, mention listing

**Files:**
- Create: `src-tauri/src/mentions.rs` (with `#[cfg(test)] mod tests`)
- Modify: `src-tauri/src/documents.rs` (`replace_links` gains a party branch; `fetch` refreshes chip labels; tests)
- Modify: `src-tauri/src/parties.rs` (`delete_party` also deletes the record's links; test)
- Modify: `src-tauri/src/lib.rs` (`mod mentions;` and register `mentions::list_party_mentions`)

**Interfaces:**
- Consumes: `parties` table (`id`, `name`, `shared`), `links(src_doc_id, dst_type, dst_id, label)`, `documents`, `cases`.
- Produces:
  ```rust
  #[derive(Serialize)] pub struct Mention { pub doc_id: i64, pub case_id: i64, pub case_code: Option<String>, pub kind: String, pub title: String }
  pub fn refresh_party_labels(conn: &Connection, content_json: &str) -> String
  pub fn party_mentions(conn: &Connection, party_id: i64, case_id: Option<i64>) -> Result<Vec<Mention>, String>
  #[tauri::command] pub fn list_party_mentions(db: State<Db>, party_id: i64, case_id: Option<i64>) -> Result<Vec<Mention>, String>
  ```
- `refresh_party_labels`: walks the TipTap JSON recursively; for every node of type `mention` whose `attrs.id` is `party:<n>` and whose party exists, sets `attrs.label` to the party's current `name`. Returns the original string unchanged when nothing changed, when the JSON is invalid, or when it is empty.
- `party_mentions`: unknown `party_id` is `Err("record not found")`. Rows are documents with a `links` row `dst_type='party'` for this record, newest `updated_at` first. Rows in `case_id` (when `Some`) are always included; rows in other cases are included only when the record's `shared = 1`. `case_code` is `Some(code)` only when the document's case differs from `case_id` (every row when `case_id` is `None`, which is only reachable when shared).

- [ ] **Step 1: Write failing tests**
  - `documents.rs`: `replace_links_accepts_party_targets_from_any_case_and_skips_missing_ones`: a note in case 1 linking to a party that is linked only to case 2 keeps the row; a link to party id 999 is dropped; the save does not error. `a_notes_backlinks_ignore_party_links`: a `links` row with `dst_type = 'party'` and `dst_id` equal to a document's id must not appear in `backlinks_to(conn, "document", that_id)`. `fetch_shows_current_party_names`: a document whose JSON contains a `party:1` mention labelled "Old" returns the party's current name after the party is renamed, a mention of missing `party:999` keeps its label, `doc:`/`src:` mentions are untouched, a mention nested inside a list item is refreshed, invalid JSON is returned unchanged. `exports_use_the_current_party_name_and_no_ids`: `export::to_markdown` and `export::to_html` over the fetched content contain the new name and neither contains `party:`.
  - `parties.rs`: `deleting_a_record_removes_its_mention_links` (a `links` row for the record disappears; links to other targets stay).
  - `mentions.rs`: `unshared_record_lists_only_the_given_cases_notes` (notes in cases 1 and 2 mention party P, unshared: with `Some(1)` only the case-1 note, `case_code == None`; the `serde_json::to_string` of the result contains neither case 2's code nor title), `shared_record_lists_other_cases_with_codes` (same data, shared: both notes, the case-2 note has `case_code == Some(code)`, the case-1 note `None`), `no_case_context_and_unshared_returns_nothing`, `no_case_context_and_shared_lists_everything_with_codes`, `unknown_record_is_an_error`.
- [ ] **Step 2: Run** `cd src-tauri && cargo test` → the new tests FAIL (items undefined). Save the failing output for the report.
- [ ] **Step 3: Implement** the signatures above. `replace_links`: add `"party" => "parties"` where, for this type only, the existence check ignores `case_id` (a plain `SELECT 1 FROM parties WHERE id = ?1`); keep the document self-link skip and the document/source same-case check. `fetch`: after reading the row, set `content_json = crate::mentions::refresh_party_labels(conn, &content_json)`. `delete_party`: after deleting the row, `DELETE FROM links WHERE dst_type = 'party' AND dst_id = ?1`. Register the command and `mod mentions;`.
- [ ] **Step 4: Run** `cd src-tauri && cargo test` → all pass, no warnings; `cargo build` clean.
- [ ] **Step 5: Commit** `feat: party mentions in notes (backend)`

---

### Task 2: `[[` picker and chips for people and companies

**Files:**
- Modify: `src/editor/links.ts`, `src/editor/extensions.ts`, `src/routes/DocumentEditor.tsx`, `src/lib/api.ts` (`LinkRef.dst_type`), `src/index.css`

**Interfaces:**
- Consumes: `api.searchParties(query: string, caseId?: number, limit?: number): Promise<PartyRow[]>` (exists), `PartyRow`, `ENTITY_KINDS`.
- Produces: `parseTargetId` returns `{ type: "document" | "source" | "party"; id: number }` for ids `doc:`, `src:`, `party:`; `LinkRef.dst_type` is `"document" | "source" | "party"`; `EditorContext` gains `searchParties: (query: string) => Promise<PartyRow[]>`.

- [ ] **Step 1: `links.ts`**: extend the id pattern and the returned type; `extractLinks` keeps one entry per distinct target and now emits `party` entries (the self-link skip stays document-only).
- [ ] **Step 2: `extensions.ts`**: make `suggestion.items` async. It returns the existing note/source matches (unchanged: filtered, first 8), then, only when the typed query is non-empty, up to 5 records from `ctx.searchParties(query)` as `{ id: "party:<id>", label: p.name, hint: <kind label from ENTITY_KINDS> }`, then the existing "Create note" row last. Guard against out-of-order results from fast typing (ignore a result whose query is no longer the latest; use a counter or compare to the most recent query). `command` inserts a `mention` node with the record's id and name exactly as for other targets. `renderHTML` adds the class `wikilink wikilink-party` for ids starting with `party:` and `wikilink` otherwise, keeping the node's `data-type`/`data-id` attributes (the click handler depends on them). Update the placeholder to "Write something, or press / for blocks and [[ to link a note, source, person or company…". Verify that the installed `@tiptap/suggestion` accepts an async `items`; if it does not, report BLOCKED with specifics.
- [ ] **Step 3: `DocumentEditor.tsx`**: provide `searchParties: (q) => api.searchParties(q, caseId, 5)` in the editor context; in `onContentClick`, a `party` target flushes pending edits then navigates to `/people?party=<id>&case=<caseId>`.
- [ ] **Step 4: `index.css`**: add `.prose-editor .wikilink-party` with a look distinct from note and source chips (use existing colour tokens) and the same hover behaviour.
- [ ] **Step 5: Run** `pnpm build` → passes.
- [ ] **Step 6: Manual checks for the user** (cannot be automated): `[[` plus a partial name lists notes and sources first, then people/companies with a kind label; Enter inserts a distinct chip; a blank `[[` shows no record rows; typing quickly does not show stale rows; clicking the chip saves and opens the record on the People page; note and source links and the create-note row behave as before; reopening the note shows the chip.
- [ ] **Step 7: Commit** `feat: mention people and companies with [[`

---

### Task 3: "Mentioned in" on the People page

**Files:**
- Modify: `src/lib/api.ts`, `src/lib/queries.ts`, `src/routes/People.tsx`

**Interfaces:**
- Consumes: Task 1's `list_party_mentions(party_id, case_id)`.
- Produces: `interface Mention { doc_id: number; case_id: number; case_code: string | null; kind: DocKind; title: string }`; `api.listPartyMentions(partyId: number, caseId?: number): Promise<Mention[]>`; `usePartyMentions(partyId: number | undefined, caseId: number | undefined)` (query key `["parties","mentions",partyId,caseId]`). `useSaveDocument` also invalidates `["parties"]` so mention lists refresh after a note is saved.

- [ ] **Step 1: Add** the type, api function (invoke args `{ partyId, caseId }`) and hook; extend `useSaveDocument`'s invalidation.
- [ ] **Step 2: `People.tsx`**: add a "Mentioned in" section to `PartyEditor` (pass `caseId` down). Each row links to `/cases/${case_id}/docs/${doc_id}` and shows the note title (or "Untitled"), kind, and the case code when `case_code` is present. Empty states: shared and none → "Not mentioned in any note."; unshared with a case context and none → "Not mentioned in this case file's notes."; unshared with no case context → "Mentions from case files are hidden. Turn on “Show which cases this appears in” to list them." (the query is still sent; the backend returns nothing).
- [ ] **Step 3: Run** `pnpm build` → passes.
- [ ] **Step 4: Manual checks for the user:** mention a person in two notes in one case; the record page (opened from the entity panel, with `?case=`) lists both. Open the same record from the sidebar with sharing off: the hidden message shows. Turn sharing on and mention them in a note in a second case: the list shows both cases' notes with codes. Delete an unlinked record: the notes keep the text and a leftover chip click shows the readable not-found message.
- [ ] **Step 5: Commit** `feat: show mentions on a record's page`

---

### Task 4: Docs and final verification

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update `CLAUDE.md`:** the Links bullet (`doc:<id>` / `src:<id>` / `party:<id>`; party targets are global, not same-case; deleting a record removes its party links by hand; chip labels for `party:` are resolved to current names inside `documents::fetch` via `mentions::refresh_party_labels`, so opening and exporting a note show current names); add `mentions` to the module list; correct the known issue "`[[` chips keep a snapshot title and go stale" to say it now applies only to note and source chips; record in the Status section that global people and companies passed a manual smoke test on 2026-11-23 and that mentions in notes are built but not yet exercised by hand, listing the checks from Tasks 2 and 3.
- [ ] **Step 2: Run** `cd src-tauri && cargo test` and `pnpm build` from the repo root; both pass; use the real test count in CLAUDE.md.
- [ ] **Step 3: Commit** `docs: update CLAUDE.md for people and companies in notes`
