# Global People and Companies Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One global record per real-world person or organization that cases reference, with a pick-or-create picker on the Connections page and a "People & Companies" directory page.

**Architecture:** A new `parties` table holds identity (kind, name, aliases, description, `shared`). The existing `entities` table stays as the per-case involvement and gains `party_id`; its `name`/`kind` columns remain as mechanical copies kept in sync by a SQL trigger, so the graph, relationships, evidence and FTS code keep working unchanged. A new `parties.rs` module owns identity CRUD, search, linking and the visibility rule (case lists exist only when `shared = 1`, enforced in Rust).

**Tech Stack:** Rust (rusqlite bundled, FTS5), Tauri 2 commands, React 19, TanStack Query, Tailwind v4, react-router 8.

**Spec:** `docs/superpowers/specs/2026-11-23-global-people-and-companies-design.md`

## Global Constraints

- This is **migration 9** (append a new entry to `MIGRATIONS` in `src-tauri/src/db.rs`; current version is 8). Never edit an existing entry.
- Do **not** drop or rebuild the `entities` table: `foreign_keys` is ON during migration, so dropping it cascade-deletes every relationship. Use `ALTER TABLE ... ADD COLUMN` plus triggers.
- Every new `#[tauri::command]` must be registered in `invoke_handler` in `src-tauri/src/lib.rs`.
- Entity kinds stay exactly `person`, `organization`, `public_body`.
- FTS rows for parties use `rowid = id*8 + 7` and `kind = 'party'` (codes 1 to 6 are taken).
- `shared` is `0` by default. Case lists and "also in N other cases" are returned **only** when `shared = 1`; this is enforced in Rust, not the UI. Error messages must never name another case.
- The switch label is exactly **"Show which cases this appears in"**.
- Names, kinds, aliases and descriptions are visible app-wide by design. The description field shows the hint: "Visible in every case file. Don't put sensitive case details here."
- Validate in Rust (kind whitelist, blank names, same-case checks, friendly errors). Logic lives in plain functions taking `&Connection`; commands are thin wrappers.
- Mutations that change shown data invalidate the matching query keys in `src/lib/queries.ts` (`["graph"]`, `["cases"]`, and the new `["parties"]`).
- Frontend: `@/` alias, Tailwind tokens from `src/index.css`, no network calls, SQLite timestamps via `parseSqlite`.
- There is no frontend test runner. Frontend tasks are verified with `pnpm build` and the manual checks listed in the task.
- Commit steps run only if the user has asked for commits in this session; otherwise leave changes uncommitted and say so. Commit messages end with the attribution line the session specifies.

## Review Focus

1. A record that is not shared must never expose which cases it is in: `search_parties`, `get_party` (including its serialized JSON) and the delete error must contain no other case's code or title.
2. Two different people with the same name must be linkable to one case; linking the same record twice to one case must fail with a readable message.
3. Renaming a record must update every involvement's `name`/`kind` and both search entries (the record's and each involvement's), so no stale copy remains in the graph or in ⌘K.
4. Migration must preserve every entity id, position (`x`,`y`), relationship and evidence row; the backfill is one record per entity.
5. Punctuation and quotes in picker queries (`O'Brien "Bob"`, `acme-corp`, a lone `"`) must not error; a blank query lists everything by name.
6. Deleting a case leaves its records; deleting a record with any involvement is refused; deleting an unlinked record succeeds.
7. A backup made at schema version 8 must restore into the version 9 app and migrate forward.

---

### Task 1: Migration 9, trigger-synced identity, FTS, and fixture fixes

**Files:**
- Modify: `src-tauri/src/db.rs` (append MIGRATIONS entry; refactor `migrate`; add tests module)
- Modify: `src-tauri/src/connections.rs` (test fixtures only: `setup()` and `entity_names_are_unique_per_case_and_kind_ignoring_case`)
- Modify: `src-tauri/src/search.rs` (test fixture at the `INSERT INTO entities` line)
- Modify: `src-tauri/src/backup.rs` (add one test)
- Grep: `grep -rn "INTO entities" src-tauri/src` and fix every fixture so it inserts a party first and sets `party_id`.

**Interfaces:**
- Produces: tables/columns `parties(id, kind, name, aliases, description, shared, created_at, updated_at)`, `entities.party_id`; `fn migrate_to(conn: &Connection, target: usize) -> rusqlite::Result<()>` in `db.rs` (private, usable from tests in `db.rs` and made `pub(crate)` for the backup test); FTS rows for parties.

- [ ] **Step 1: Write failing tests in `db.rs` (`#[cfg(test)] mod tests`)**
  - `v9_backfills_one_party_per_entity_and_keeps_the_graph`: open an in-memory connection, `PRAGMA foreign_keys=ON`, `migrate_to(&conn, 8)`, insert 1 case, 3 entities (with `x`,`y` on one), 1 source, 1 relationship, 1 `relationship_sources` row; then `migrate_to(&conn, 9)`. Assert: `parties` count == 3; every `entities.party_id == entities.id`; party `name`/`kind` equal the entity's; all `shared == 0`; relationship and evidence counts unchanged; the positioned entity keeps `x`,`y`; `PRAGMA user_version == 9`; an FTS query `SELECT kind FROM search WHERE search MATCH '"northbank"*' AND kind='party'` returns a row.
  - `renaming_a_party_updates_entities_and_search`: after migration, `UPDATE parties SET name='Renamed', kind='person' WHERE id=1` makes `entities.name/kind` for `party_id=1` equal the new values, a `kind='entity'` search hit for `Renamed`, and a `kind='party'` hit for `Renamed` (and none for the old name).
  - `entity_insert_requires_a_party`: `INSERT INTO entities (case_id, kind, name) VALUES (1,'person','X')` fails; the same with a valid `party_id` succeeds.
  - `two_parties_may_share_a_name_in_one_case_but_one_party_links_once`: two parties both named "John Smith" each link to case 1 (ok); linking party 1 to case 1 a second time fails on the unique index.
- [ ] **Step 2: Run** `cd src-tauri && cargo test db::tests` → FAIL (`migrate_to` and `parties` undefined).
- [ ] **Step 3: Implement** `fn migrate_to(conn, target: usize)` (the existing loop bounded to `target`; `migrate` calls it with `MIGRATIONS.len()`), and the new migration entry:
  - `CREATE TABLE parties (...)` with `shared INTEGER NOT NULL DEFAULT 0 CHECK (shared IN (0,1))`.
  - Backfill `INSERT INTO parties (id, kind, name) SELECT id, kind, name FROM entities;` (party id = entity id).
  - `ALTER TABLE entities ADD COLUMN party_id INTEGER REFERENCES parties(id);` then `UPDATE entities SET party_id = id;`.
  - `DROP INDEX entities_unique;` `CREATE UNIQUE INDEX entities_case_party ON entities(case_id, party_id);` `CREATE INDEX entities_party ON entities(party_id);`
  - `entities_require_party` BEFORE INSERT trigger that `RAISE(ABORT, ...)` when `new.party_id IS NULL`.
  - `parties_sync_entities` AFTER UPDATE OF kind, name ON parties: `UPDATE entities SET kind=new.kind, name=new.name WHERE party_id=new.id`.
  - FTS: backfill and `search_parties_ai/au/ad` triggers using `id*8+7`, `kind='party'`, `case_id` NULL, title = name, body = `aliases || ' ' || description`.
- [ ] **Step 4: Update fixtures** found by the grep: insert parties (explicit ids) before entities and set `party_id`. Replace the `entity_names_are_unique_per_case_and_kind_ignoring_case` test in `connections.rs` with one named `same_party_cannot_be_linked_twice_but_same_name_can_repeat` asserting the behaviour in Review Focus 2.
- [ ] **Step 5: Add `restoring_a_v8_backup_migrates_forward` to `backup.rs` tests**: build a DB with `migrate_to(.., 8)` and one entity, write a backup zip with manifest `schema_version: 8` (reuse the existing `zip_with` helper), run `stage_restore` + `apply_pending`, `db::open` the result; assert `user_version == 9` and one party exists.
- [ ] **Step 6: Run** `cd src-tauri && cargo test` → all pass (the 23 existing plus the new ones).
- [ ] **Step 7: Commit** `feat: migration 9 adds global parties table`

---

### Task 2: `parties.rs` identity CRUD and search

**Files:**
- Create: `src-tauri/src/parties.rs` (with `#[cfg(test)] mod tests`)
- Modify: `src-tauri/src/lib.rs` (`mod parties;` and register `parties::create_party`, `update_party`, `delete_party`, `search_parties`)
- Modify: `src-tauri/src/search.rs` (make `to_match` `pub(crate)`)

**Interfaces:**
- Produces:
  ```rust
  pub const KINDS: &[&str] = &["person", "organization", "public_body"];
  #[derive(Deserialize)] pub struct PartyInput { pub kind: String, pub name: String, pub aliases: Option<String>, pub description: Option<String>, pub shared: Option<bool> }
  #[derive(Serialize)] pub struct PartyRow { pub id: i64, pub kind: String, pub name: String, pub aliases: String, pub description: String, pub shared: bool, pub in_this_case: bool, pub other_case_count: Option<i64> }
  pub fn create_party(conn: &Connection, input: &PartyInput) -> Result<i64, String>
  pub fn update_party(conn: &Connection, id: i64, input: &PartyInput) -> Result<(), String>   // shared: None leaves it unchanged
  pub fn delete_party(conn: &Connection, id: i64) -> Result<(), String>
  pub fn search_parties(conn: &Connection, q: &str, case_id: Option<i64>, limit: i64) -> rusqlite::Result<Vec<PartyRow>>
  ```
  Commands: `create_party(db, input) -> i64`, `update_party(db, id, input)`, `delete_party(db, id)`, `search_parties(db, query: String, case_id: Option<i64>, limit: Option<i64>) -> Vec<PartyRow>` (default limit 8).
- `other_case_count`: `Some(n)` only when `shared = 1`, where n is the number of cases other than `case_id` (all cases when `case_id` is `None`); otherwise `None`. `in_this_case` is true when an involvement exists in `case_id`, regardless of `shared`.

- [ ] **Step 1: Write failing tests** (fixtures insert cases, parties and entities with SQL):
  - `create_validates_kind_and_name`: blank/whitespace name and kind `"company"` are `Err`; name and fields are trimmed; `shared` defaults to false.
  - `same_name_records_are_allowed`: two `person` "John Smith" with descriptions "Mayor" and "Accountant" both create.
  - `search_matches_name_alias_and_description_by_prefix`: a record with name "Acme Corp", aliases "Acme Corporation", description "Shell company": queries `acm`, `corporat`, `shell` each return it.
  - `search_with_punctuation_does_not_error`: `O'Brien "Bob"`, `acme-corp`, `"` all return `Ok`.
  - `blank_query_lists_everything_by_name_nocase`: returns all records ordered by name ignoring case, respecting `limit`.
  - `case_info_appears_only_for_shared_records`: a shared record in cases 1 and 2 and an unshared record in cases 1 and 2; `search_parties(.., Some(1), ..)` gives the shared one `other_case_count == Some(1)` and the unshared one `None`, both `in_this_case == true`.
  - `update_party_changes_fields_and_toggles_shared`; unknown id gives `Err("record not found")`.
  - `delete_is_blocked_while_linked_with_a_generic_message`: `Err` text contains neither "Case B" nor its code; after removing the involvements the delete succeeds.
- [ ] **Step 2: Run** `cd src-tauri && cargo test parties::tests` → FAIL.
- [ ] **Step 3: Implement** the functions above in `parties.rs`. `search_parties`: when `to_match(q)` is `Some`, `SELECT ... FROM search s JOIN parties p ON p.id = s.ref_id WHERE search MATCH ?1 AND s.kind = 'party' ORDER BY bm25(search, 5.0, 1.0)`; otherwise list `parties` ordered by `name COLLATE NOCASE`. Compute `in_this_case` and `other_case_count` with correlated subqueries over `entities`. Commands are thin wrappers using `db.0.lock()`.
- [ ] **Step 4: Run** `cargo test` → all pass.
- [ ] **Step 5: Commit** `feat: parties module with identity CRUD and search`

---

### Task 3: Linking, detail with the visibility rule, and connections changes

**Files:**
- Modify: `src-tauri/src/parties.rs` (add linking and detail)
- Modify: `src-tauri/src/connections.rs` (`Entity` gains `party_id`; remove `create_entity` command and its `KINDS`; `update_entity` becomes notes-only; make `touch_case` `pub(crate)`; `load_graph` selects `party_id`; update tests)
- Modify: `src-tauri/src/lib.rs` (register `link_party`, `create_party_and_link`, `get_party`; remove `connections::create_entity`)

**Interfaces:**
- Consumes: Task 2's `PartyInput`, `create_party`, `KINDS`.
- Produces:
  ```rust
  #[derive(Serialize)] pub struct CaseRef { pub case_id: i64, pub code: String, pub title: String, pub connections: i64 }
  #[derive(Serialize)] pub struct PartyDetail { pub id: i64, pub kind: String, pub name: String, pub aliases: String, pub description: String, pub shared: bool, pub in_this_case: Option<CaseRef>, pub cases: Option<Vec<CaseRef>> }
  pub fn link_party(conn: &Connection, case_id: i64, party_id: i64) -> Result<i64, String>        // returns the new entity id
  pub fn create_and_link(conn: &Connection, case_id: i64, input: &PartyInput) -> Result<i64, String> // returns the entity id; one transaction
  pub fn get_party(conn: &Connection, id: i64, case_id: Option<i64>) -> Result<PartyDetail, String>
  ```
  Commands: `link_party(db, case_id, party_id) -> i64`, `create_party_and_link(db, case_id, input) -> i64`, `get_party(db, id, case_id: Option<i64>) -> PartyDetail`, and in `connections.rs` `update_entity(db, id: i64, notes: String)`. `Entity` gets `pub party_id: i64`. `connections` = relationships touching that involvement.
- Rule: `cases` is `Some` only when `shared = 1`; `in_this_case` is `Some` whenever the record is linked to `case_id`.

- [ ] **Step 1: Write failing tests** in `parties.rs`:
  - `link_party_copies_identity_into_the_involvement`: entity `kind`/`name` equal the record's; `load_graph(case)` returns it with the right `party_id`.
  - `same_party_twice_is_rejected_with_a_readable_message`: error contains "already"; two same-named records link fine.
  - `link_rejects_missing_case_or_party`: both `Err`.
  - `detail_hides_cases_unless_shared`: for an unshared record linked in cases 1 and 2, `get_party(.., Some(1))` has `cases == None` and `in_this_case == Some`; `serde_json::to_string` of it contains neither "Case B" nor case B's code. After `update_party(.. shared: Some(true))`, `cases` lists both with correct `connections` counts.
  - `deleting_a_case_keeps_its_records`: delete case 2; its parties still exist; `delete_party` on one then succeeds.
  - `create_and_link_is_atomic`: blank name returns `Err` and the `parties` count is unchanged.
  - In `connections.rs`: `update_entity_only_changes_notes` (name and kind untouched).
- [ ] **Step 2: Run** `cargo test` → FAIL (and fix compile breaks from the removed `create_entity`).
- [ ] **Step 3: Implement** the signatures above. `link_party` inserts `INSERT INTO entities (case_id, kind, name, notes, party_id)` copying `kind`/`name` from the record, maps a unique-constraint failure to "this case file already has that person or organization", then calls `touch_case`. `create_and_link` wraps `create_party` + `link_party` in a transaction (`conn.unchecked_transaction()`).
- [ ] **Step 4: Run** `cd src-tauri && cargo test` → all pass; `cargo build` clean (no unused-code warnings from the removed command).
- [ ] **Step 5: Commit** `feat: link records to cases with the shared visibility rule`

---

### Task 4: Frontend API types and queries

**Files:**
- Modify: `src/lib/api.ts`
- Modify: `src/lib/queries.ts`

**Interfaces:**
- Produces (TypeScript mirrors of Task 2 and 3, field names identical): `PartyInput`, `PartyRow`, `CaseRef`, `PartyDetail`; `Entity.party_id: number`; `HitKind` gains `"party"`.
  - `api.searchParties(query: string, caseId?: number, limit?: number): Promise<PartyRow[]>`, `api.getParty(id: number, caseId?: number): Promise<PartyDetail>`, `api.createParty(input)`, `api.updateParty(id, input)`, `api.deleteParty(id)`, `api.linkParty(caseId, partyId): Promise<number>`, `api.createPartyAndLink(caseId, input): Promise<number>`, `api.updateEntity(id: number, notes: string)`. Remove `api.createEntity` and the old `EntityInput`.
  - Hooks: `usePartySearch(query: string, caseId: number | undefined, limit?: number)` (key `["parties","search",query,caseId,limit]`), `useParty(id: number | undefined, caseId: number | undefined)` (key `["parties","detail",id,caseId]`), `useLinkParty`, `useCreatePartyAndLink`, `useUpdateParty`, `useDeleteParty`, and `useUpdateEntity` now taking `{ id, notes }`.
- Invalidation: every party mutation invalidates `["parties"]`, `["graph"]` and `["cases"]` (names are copied into involvements).

- [ ] **Step 1: Implement** the types, `api` functions and hooks above, following the existing `useGraphMutation` pattern.
- [ ] **Step 2: Run** `pnpm build` → fails only in `Modals.tsx`/`Panels.tsx` (they still use the removed `createEntity`/`EntityInput`); that is fixed in Task 5. Expected errors name those two files and nothing else.
- [ ] **Step 3: Commit** (together with Task 5, since the tree does not build until then).

---

### Task 5: Pick-or-create dropdown and entity panel

**Files:**
- Create: `src/connections/PartyPicker.tsx`
- Modify: `src/connections/Modals.tsx` (`EntityModal`)
- Modify: `src/connections/Panels.tsx` (`EntityPanel`)
- Modify: `src/routes/Connections.tsx` (pass `caseId` to `EntityPanel`)

**Interfaces:**
- Produces: `PartyPicker({ caseId, query, onPick }: { caseId: number; query: string; onPick: (party: PartyRow) => void })` renders the inline results list under the name field (does not own the input). `EntityPanel` gains a `caseId: number` prop.

- [ ] **Step 1: `PartyPicker`** uses `usePartySearch(useDeferredValue(query), caseId, 8)` and renders nothing for a blank query. Each row: name, kind label (from `ENTITY_KINDS`), description; `"also in N other cases"` only when `other_case_count` is a number greater than 0; a row with `in_this_case` is disabled and reads "already in this case". Keyboard: ArrowUp/ArrowDown move the highlight, Enter on a highlighted row calls `onPick`, Escape closes the list without closing the modal (`stopPropagation`).
- [ ] **Step 2: `EntityModal`**: keep Type and Notes; add Description (with the hint text from Global Constraints); name input drives `PartyPicker`. Picking a row calls `useLinkParty` then `onCreated(entityId)` and closes. The primary button reads "Create new" when no row is highlighted and calls `useCreatePartyAndLink` with `{ kind, name, description }` (notes saved afterwards with `useUpdateEntity` if non-empty).
- [ ] **Step 3: `EntityPanel`**: identity fields (Type, Name, Aliases, Description) load from `useParty(entity.party_id, caseId)` and save through `useUpdateParty`, with the line "Changes to type, name, aliases and description apply in every case file that uses this record." Notes save through `useUpdateEntity({ id, notes })`. Add a link "Open in People & Companies" to `/people?party=${entity.party_id}&case=${caseId}`.
- [ ] **Step 4: Run** `pnpm build` → passes.
- [ ] **Step 5: Manual check** (the user drives `pnpm tauri dev`; restart it first so migration 9 runs): Connections → "+ Add entity": typing an existing name shows it in the dropdown and linking it adds a node; typing a new name and clicking "Create new" adds one; a second record with the same name in one case is allowed; linking the same record twice shows "already in this case". Renaming from the panel updates the graph label.
- [ ] **Step 6: Commit** `feat: pick-or-create picker for entities`

---

### Task 6: People & Companies page, sidebar item, ⌘K hits

**Files:**
- Create: `src/routes/People.tsx`
- Modify: `src/App.tsx` (route `people`), `src/components/layout/Sidebar.tsx` (item after "Connections": `{ to: "/people", label: "People & Companies" }`), `src/components/CommandPalette.tsx` (`KIND_LABEL.party = "Person / org"`; `destination` returns `/people?party=${h.ref_id}` for `"party"`)

**Interfaces:**
- Consumes: `usePartySearch`, `useParty`, `useUpdateParty`, `useDeleteParty` (Task 4). Reads `?party=<id>` and optional `?case=<id>` from the URL.

- [ ] **Step 1: Implement `People.tsx`**: left column is a search box plus a list (`usePartySearch(query, caseId, 500)`) showing name, kind, description; right column is the selected record (`useParty(partyId, caseId)`): editable Type, Name, Aliases, Description with Save; a checkbox labelled **"Show which cases this appears in"** saved through `useUpdateParty`; when `detail.cases` is present a list of cases (code, title, connections) each linking to `/cases/:case_id`; when absent the text "Case details are hidden. Turn on “Show which cases this appears in” to list them."; when `detail.in_this_case` is present a line "In this case: N connections"; a Delete button showing the returned error text. Empty state when nothing is selected.
- [ ] **Step 2: Wire** the route, sidebar item and palette changes.
- [ ] **Step 3: Run** `pnpm build` → passes.
- [ ] **Step 4: Manual check:** the sidebar item opens the list; selecting a record shows details; with the switch off no case names appear anywhere on the page; turning it on lists every case that links it; ⌘K finds a record by name and Enter opens it on this page; Delete on a linked record shows a readable error that names no case.
- [ ] **Step 5: Commit** `feat: People & Companies page`

---

### Task 7: Docs and final verification

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update `CLAUDE.md`:** migrations "currently 9"; add the `parties` module to the module list; describe the model (global `parties`, `entities` as per-case involvement with trigger-synced `name`/`kind`, `party_id` required by trigger, `shared` rule enforced in Rust, FTS code 7); remove "Entities are hand-created and per case" from known issues and add "Entities are still not linked to notes via `[[`" and "No merge action for duplicate records yet"; replace the "Next planned work" line.
- [ ] **Step 2: Run** `cd src-tauri && cargo test` and `pnpm build` → both pass; report the counts.
- [ ] **Step 3: Tell the user** to back up first (Settings → Back up…) because the next `pnpm tauri dev` run migrates their dev database to version 9, then run the manual checks from Tasks 5 and 6 plus a backup/restore round trip.
- [ ] **Step 4: Commit** `docs: update CLAUDE.md for global people and companies`
