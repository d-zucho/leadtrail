# Fieldnotes

Local-first Tauri 2 desktop app for investigative journalists, researchers and investigators: case files, sources, evidence uploads, an article/note editor, lead inbox, connections graph, research dashboard. Feels like Notion + Obsidian. The repo directory is still called `leadtrail` (from the scaffold); the product is "Fieldnotes".

## Status: all 11 planned slices are built; nothing has been tested in the running app

Verification so far is only `pnpm build` (tsc + vite) and `cargo test` (23 passing). `pnpm tauri dev` launched cleanly with no Rust errors, but **no UI has been exercised by a human or by us**. The next job is a manual smoke test and fixing what breaks, not new features.

Smoke-test order (riskiest first):
1. Editor: type `[[` (note/source picker) and `/` (slash menu) in a note.
2. Connections: add two entities, connect them, drag a node, navigate away and back; the position must persist.
3. ⌘K search finds a note by partial title; Enter opens it.
4. Settings → Back up…, then Restore from backup… (app restarts; data intact; a `pre-restore-*` folder appears in the app data dir).
5. Publication queue: article status changes; Markdown/HTML export; PDF opens a print view in the browser.

## Commands
- `pnpm tauri dev` run the app. `pnpm build` type-check + bundle. `cd src-tauri && cargo test` Rust tests.
- pnpm is strict: import only packages listed in `package.json` (e.g. `@tiptap/core` had to be added directly).

## Architecture
- **Rust owns the data.** `rusqlite` (bundled, has FTS5). The frontend only calls `invoke` (typed wrappers in `src/lib/api.ts`). Commands return `Result<T, String>`.
- Logic lives in plain functions taking `&Connection` so tests use `db::open(Path::new(":memory:"))`; `#[tauri::command]` fns are thin wrappers. State: `db::Db(Mutex<Connection>)`, `sources::Vault(PathBuf)`.
- **Migrations** are an append-only `MIGRATIONS` array in `src-tauri/src/db.rs`, tracked by `PRAGMA user_version` (currently 8, see `db::schema_version()`). Never edit an existing entry; add a new one. Every new command must be registered in `invoke_handler` in `lib.rs`.
- Data dir = Tauri `app_data_dir`: `fieldnotes.db` + `vault/evidence/<case_id>/<hash12>-<name>`. Evidence is SHA-256 hashed, verified after copy, deduped per case.
- Modules (`src-tauri/src/`): `cases`, `sources`, `leads`, `documents` (also article status + export commands), `questions`, `activity`, `connections`, `search`, `export` (TipTap JSON → Markdown/HTML), `backup`, `db`.
- **Search:** FTS5 table `search` kept in sync by SQL triggers on cases/sources/leads/documents/questions/entities (rowid = `id*8 + kind_code`). New searchable tables need triggers too.
- **Links:** editor mentions have ids `doc:<id>` / `src:<id>`; on save the frontend extracts them and `replace_links` keeps only same-case, existing, non-self targets. Polymorphic targets have no FK, so delete code removes `links` rows by hand.
- **Restore** never touches the live DB: it validates and stages the zip, restarts, and `backup::apply_pending` swaps it in before the DB opens.
- Frontend: React 19, TS, Vite 8, Tailwind v4 (tokens in `@theme` in `src/index.css`), `react-router` 8 with HashRouter, TanStack Query, zustand (`src/store/ui.ts`), `@/` alias to `src`. Fonts are bundled via `@fontsource-variable/*` (no network calls; keep it that way).
- TipTap 3 editor in `src/editor/` (StarterKit, task lists, `[[` Mention with `allowSpaces`, custom `/` Suggestion). Autosave is 800 ms debounced with refs for a stable `flush`; `useDocument` has `staleTime: Infinity` so the editor is never overwritten behind its back.
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
- Entities are hand-created and per case; they are not linked to notes or shared across cases.
- Search matches words only (punctuation dropped); sources/questions results land on the case page.
- Dashboard has no names in the greeting and no question owners; `src/routes/Placeholder.tsx` is now unused.
