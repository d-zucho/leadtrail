# Global people and companies — design

Date: 2026-11-23. Status: approved by the user (including the switch label "Show which cases this appears in").

## Intent and why

Fieldnotes is a local-first desktop app for investigative journalists and researchers. Today a person or organization exists only as a per-case "entity" (`entities.case_id`). If the same company appears in three investigations, the user creates it three times and nothing ties the copies together.

The goal: **one global record per real-world person or organization, which cases reference.** The user can see which people and companies are tied to several investigations, and duplicates are prevented at the point of entry.

The countervailing concern is confidentiality. Investigators often wall cases off from each other. A global record must not leak that another case exists, or what it says, unless the user opts in.

## Decisions made with the user

1. **Global record + per-case involvement (hybrid).** The global record holds identity only. Everything about a record's role in a case stays in that case.
2. **Every global record is visible everywhere by name; its case list is opt-in.** Like linking to any note in Obsidian, the picker, directory and search show every global record (name, kind, aliases, description) regardless of case. This is what prevents duplicates. Which cases a record appears in is hidden by default. Each record has a `shared` switch, off by default; only when on does the UI show the cases it appears in. (The user chose this over a stricter rule where unshared records are invisible outside their own cases.)
3. **Pick-or-create when adding an entity to a case.** Search existing global records first; create a new one only if none fits. Chosen UI: an **inline dropdown** under the name field on the Connections page (not a modal).
4. **A one-line description on each global record** (for example "Mayor, Springfield") so same-name people can be told apart in the dropdown and the directory.
5. **A "People & Companies" sidebar page**: searchable list of all global records; selecting one shows its details, the "Shared across cases" switch, and either only this case's involvement (sharing off) or a clickable list of every case it appears in (sharing on).
6. **Existing data migrates one to one.** Each existing entity becomes one global record plus its involvement. No automatic merging of same-name entities.

## Model

- **Global record** (new table, name is the builder's call): `kind` (keep the existing whitelist `person`, `organization`, `public_body`), `name`, `aliases`, `description` (one line), `shared` (0/1, default 0), timestamps.
- **Involvement** = the existing `entities` table. It keeps `case_id`, `notes`, `x`, `y`, and is still what `relationships` point at, so the graph code and the evidence-strength edges keep working. It gains a reference to the global record.
- A case may link a given global record **at most once** (replaces the current uniqueness of `(case_id, kind, name)` with uniqueness of `(case_id, global record)`). Two different people with the same name must be linkable in one case.

## Behavior

- **Adding an entity (Connections page):** typing searches **all** global records by name, alias and description. Each result row shows name, kind, description. A result shows "also in N other cases" only if that record is shared; otherwise it shows no case information at all. The last row is "Create new" with a kind choice. Choosing a result creates an involvement in the current case. Linking a record already in this case is rejected with a friendly error.
- **Directory page:** list with search; detail panel with name, kind, aliases, description, the shared switch, and this case's involvement counts (connections, notes). With sharing on: the list of cases, each opening that case. Editing identity fields here updates the record everywhere.
- **Renaming** a global record is visible in every case that links it.
- **Deleting:** removing an entity from a case deletes only the involvement. A global record can be deleted only when it has no involvements. Deleting a case removes its involvements and leaves its global records (they may become unreferenced; the directory still lists them). *(Assistant's call, not yet confirmed with the user.)*
- **Search (⌘K):** global records are searchable by name. Case-scoped involvement search keeps working as today.

## Constraints

- Append-only migrations: this is **migration 9** (current schema version is 8). Never edit an existing migration. Register any new command in `invoke_handler` in `lib.rs`.
- Logic as plain functions taking `&Connection`, with `#[tauri::command]` wrappers; validate in Rust (kind whitelist, same-case checks, friendly constraint errors), not only in the UI.
- Search is an FTS5 table kept in sync by SQL triggers; a new searchable table needs its own triggers. `rowid = id*8 + kind_code`, and codes 1 to 6 are already used, so the new table needs code 7 (or 0).
- When a mutation changes data shown elsewhere, invalidate those query keys in `src/lib/queries.ts`.
- Backup and restore must keep working. A backup made at schema version 8 must still restore into the new app and migrate forward. The builder must verify how `backup::unpack` validates the schema version.
- **The `shared` switch is a UI privacy feature, not access control.** The database and evidence are unencrypted on disk (known gap, see CLAUDE.md). The doc and UI copy must not imply otherwise.

## Non-goals (this slice)

- A merge action for duplicate records (a later slice; migration deliberately does not auto-merge).
- A "view in graph" shortcut from the directory page.
- Linking entities to notes via `[[` mentions, or sharing entities' notes or evidence across cases.
- Relationships between entities in different cases.
- Encryption at rest.

## Anti-goals (a failure even if it technically works)

- Revealing **which cases** a record appears in, or any case content (involvement notes, connections, evidence), for a record that is not shared. This includes the dropdown, the directory page, search results and error messages. (A record's own name, kind, aliases and description are visible app-wide by design, so the UI should hint that descriptions should not contain sensitive case details.)
- Silently combining two different people or companies.
- Losing or corrupting existing entities, positions, relationships or evidence during migration.
- Breaking the existing graph, ⌘K search or backup and restore.

## Calls made by the assistant (not yet confirmed by the user)

- **Directory scope:** a global page listing all records. "In this case" appears only when opened from a case. Records with no involvements stay visible.
- **Editing and deleting:** any record can be edited from anywhere, and the edit applies everywhere. Delete is blocked while the record has any involvement, with a generic message that does not name other cases.
- **Migration:** the old `(case_id, kind, name)` uniqueness is dropped and replaced by `(case_id, global record)` in the same migration. Aliases and descriptions start empty. A record's kind stays editable, as `update_entity` allows today.
- **Switch label:** because names are now visible everywhere, "Shared across cases" would mislead. Suggested label: "Show which cases this appears in". The earlier mockup label was approved by the user, so this rename needs their OK.

## Left to the builder

- Table and column names, how aliases are stored (and indexed in FTS), and how `entities.name` and `entities.kind` relate to the global record (keep as a synced copy, or read through a join). Pick one source of truth so they cannot drift.
- Exact UI details of the dropdown and directory page, within the choices above.
- Test plan. At minimum: migration of existing entities, picker search, the shared-flag visibility rule, rejecting a duplicate link in one case, and delete rules.
