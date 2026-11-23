# Fieldnotes

Local-first Tauri 2 desktop app for investigative journalists, researchers and investigators: case files, sources, evidence uploads, an article/note editor, lead inbox, connections graph, research dashboard. Feels like Notion + Obsidian. The repo directory is still called `leadtrail` (from the scaffold); the product is "Fieldnotes".

## Status: the 11 original slices are built and manually smoke-tested

Verification: `pnpm build` (tsc + vite), `cargo test` (45 passing), and a manual smoke test in `pnpm tauri dev` (2026-11-23) of the five riskiest flows. All passed:

1. Editor: `[[` picker (filters as you type, inserts a chip), `/` slash menu, links show up on the linked note after save.
2. Connections: entities, edges, node drag; position persists after navigating away.
3. ⌘K search: partial title, text inside a note body, and a renamed note's new title.
4. Backup → Restore: DB and evidence files come back. The backup zip holds `manifest.json`, `fieldnotes.db` and `vault/evidence/...` (evidence only appears if files were uploaded; `manifest.json` has an `evidence_files` count).
5. Publication queue: status changes; Markdown, HTML and PDF export all keep formatting.

Open issues from the smoke test:
- **Restore relaunch crashes in dev.** After Restore, `app.restart()` relaunches the binary and the new process aborts (SIGABRT) about a second later, inside an uncaught Objective-C exception in an Apple Event handler (tao/AppKit; no frames from our code). Data is restored correctly. Seen on macOS 27.0 launching from the VS Code terminal. Not yet checked in a built `.app` (`pnpm tauri build --debug`). If it also crashes there, replace `app.restart()` in `backup::restore_backup` with `app.exit(0)` and a "reopen Fieldnotes" message.
- Not yet confirmed: that the `pre-restore-*` folder appears in the app data dir after a restore.
- Not exercised: restoring an invalid zip (should be refused with data untouched; covered only by the `rejects_bad_archives` Rust test).

Global people & companies (shared `parties`, picker, directory page) is built but its UI has not been exercised by hand yet. Next job: smoke-test it, covering the Connections picker, the People & Companies page, the shared switch, a rename propagating to the graph, ⌘K finding a record, and a backup/restore round trip.

## Commands

- `pnpm tauri dev` run the app. `pnpm build` type-check + bundle. `cd src-tauri && cargo test` Rust tests.
- pnpm is strict: import only packages listed in `package.json` (e.g. `@tiptap/core` had to be added directly).

## Architecture

- **Rust owns the data.** `rusqlite` (bundled, has FTS5). The frontend only calls `invoke` (typed wrappers in `src/lib/api.ts`). Commands return `Result<T, String>`.
- Logic lives in plain functions taking `&Connection` so tests use `db::open(Path::new(":memory:"))`; `#[tauri::command]` fns are thin wrappers. State: `db::Db(Mutex<Connection>)`, `sources::Vault(PathBuf)`.
- **Migrations** are an append-only `MIGRATIONS` array in `src-tauri/src/db.rs`, tracked by `PRAGMA user_version` (currently 9, see `db::schema_version()`). Never edit an existing entry; add a new one. Every new command must be registered in `invoke_handler` in `lib.rs`.
- Data dir = Tauri `app_data_dir`: `fieldnotes.db` + `vault/evidence/<case_id>/<hash12>-<name>`. Evidence is SHA-256 hashed, verified after copy, deduped per case.
- Modules (`src-tauri/src/`): `cases`, `sources`, `leads`, `documents` (also article status + export commands), `questions`, `activity`, `connections`, `parties`, `search`, `export` (TipTap JSON → Markdown/HTML), `backup`, `db`.
- **Search:** FTS5 table `search` kept in sync by SQL triggers on cases/sources/leads/documents/questions/entities/parties (rowid = `id*8 + kind_code`). New searchable tables need triggers too.
- **Links:** editor mentions have ids `doc:<id>` / `src:<id>`; on save the frontend extracts them and `replace_links` keeps only same-case, existing, non-self targets. Polymorphic targets have no FK, so delete code removes `links` rows by hand.
- **Restore** never touches the live DB: it validates and stages the zip, restarts, and `backup::apply_pending` swaps it in before the DB opens.
- Frontend: React 19, TS, Vite 8, Tailwind v4 (tokens in `@theme` in `src/index.css`), `react-router` 8 with HashRouter, TanStack Query, zustand (`src/store/ui.ts`), `@/` alias to `src`. Fonts are bundled via `@fontsource-variable/*` (no network calls; keep it that way).
- TipTap 3 editor in `src/editor/` (StarterKit, task lists, `[[` Mention with `allowSpaces`, custom `/` Suggestion). Autosave is 800 ms debounced with refs for a stable `flush`; `useDocument` has `staleTime: Infinity` so the editor is never overwritten behind its back.
- **People & companies:** global `parties` table (identity only: kind, name, aliases, description, `shared`). `entities` is the per-case involvement and has a required `party_id` (BEFORE INSERT trigger) with a unique index on (case_id, party_id); `entities.name`/`kind` are copies kept in sync by a trigger when a party is renamed. Never rebuild/drop `entities` in a migration (foreign_keys is ON; it would cascade-delete relationships). `update_entity` is notes-only; entities are created via `parties::create_and_link`/`link_party` (Tauri commands are in `parties::commands`).
- **Visibility rule:** a record's name/kind/aliases/description is visible app-wide (picker, directory, ⌘K); the case list and case content are hidden on the People page and in the picker unless `shared = 1`, enforced in Rust (`get_party`, `search_parties`). ⌘K search still returns per-case hits (including `entity` hits showing case codes, labelled "In case graph"), so `shared` does not hide a record from ⌘K. Switch label: "Show which cases this appears in". UI privacy feature, not access control: the DB is unencrypted.
- Parties are in FTS with rowid `id*8+7`, kind `party`. UI: Connections "+ Add entity" is a pick-or-create inline dropdown (`PartyPicker`); People & Companies sidebar page at `/people` (`?party=`, `?case=`).
- Graph: `@xyflow/react` **v13**, which differs from v12: no `applyNodeChanges`; use `changes.applyTo(nodes)`. Node moves are mirrored into the query cache so stale x/y never overwrite them. Edge colour = evidence strength (green verified, amber unverified, grey dashed none).
- Heavy routes (`DocumentEditor`, `Connections`) are lazy-loaded in `App.tsx`.
- SQLite timestamps are UTC with no zone marker; use `parseSqlite` in `src/lib/format.ts`.

## Conventions

- Validate in Rust (status/kind whitelists, same-case checks, friendly constraint errors), not only in the UI.
- When a mutation changes data shown elsewhere, invalidate those query keys in `src/lib/queries.ts` (e.g. `useDeleteCase` fans out to many keys; the `["queue"]` key for publication).
- Keep exports publish-safe: internal `[[links]]` and source mentions export as plain text; only http(s)/mailto links survive.

## Known gaps / where to continue

Deliberately left out:

- **Encryption at rest (SQLCipher) and app lock.** A lock without encryption is cosmetic; this needs its own slice (toolchain change, migrating existing data, passphrase-loss policy, encrypting evidence files, and the FTS index holds full note text). FileVault/BitLocker is the interim answer; Settings says backups are unencrypted.
- Configurable vault location (data dir is fixed; Settings shows the path and can reveal it).
- One-click PDF: PDF currently opens an HTML print view (`vault/exports/`, deleted at startup) in the browser.

Small known issues:

- Case codes are `FN-{MAX(id)+1}`; deleting the newest case reuses its code.
- `[[` chips keep a snapshot title and go stale after a rename/delete of the target.
- Leads can't be edited after capture; promoted leads become only link/note sources.
- Source modal doesn't show which notes cite the source (backend `list_source_citations` exists).
- Article status isn't shown in the case's document list (only in the editor and the Publication queue). Editing a published article doesn't change its status.
- Notes and articles share one editor; no image support in the editor or exports.
- Entities are still not linked to notes via `[[`; no merge action for duplicate records yet; no "view in graph" shortcut from the directory.
- Search matches words only (punctuation dropped); sources/questions results land on the case page.
- Dashboard has no names in the greeting and no question owners; `src/routes/Placeholder.tsx` is now unused.
