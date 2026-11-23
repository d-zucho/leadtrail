# People and companies inside notes — design

Date: 2026-11-23. Status: draft for approval.

## Intent and why

Fieldnotes already lets a note link to other notes and sources with `[[`. People and companies (global `parties`, see `2026-11-23-global-people-and-companies-design.md`) live in the directory and the connections graph, but a note cannot mention them. The goal is the Obsidian effect: while writing, mention a person or company; later, from that person's record, see every note that mentions them.

## Decisions made with the user

1. **`[[` lists people and companies too**, in the same picker as notes and sources. They are grouped and each row carries a small kind label (Person, Organization, Public body). Picking one inserts a chip with the record's name, visually distinct from note and source links.
2. **A mention is stored as a link**, exactly like note and source links: saving the note records it so backlinks work. The chip id is `party:<id>`.
3. **The record page shows "Mentioned in"**: the notes that mention it, each opening that note.
4. **Mentioning someone does not add them to the graph.** Only records already linked to a case appear in that case's graph.
5. **Names are live**: renaming a record updates what its chips show when a note is opened, and what exports show.
6. **Exports** treat the chip as plain text (the record's current name), like other internal links; no `party:` ids may leak.
7. **The existing privacy switch controls cross-case mentions.** On a record's page, notes from the case the user came from (`?case=`) are always listed; notes from other cases are listed only when the record's "Show which cases this appears in" switch is on.

## Behavior

- Typing `[[` queries notes, sources and records. Records are searched app-wide by name, aliases and description (same visibility as the existing picker). Records appear after notes and sources, up to a small fixed number of rows, each labelled with its kind.
- Inserting a record chip does not require the record to be linked to the current case.
- On save, the frontend extracts `party:<id>` mentions with the other links. `replace_links` accepts `dst_type = 'party'` when the record exists. Unlike notes and sources, a party target does **not** need to be in the same case, because records are global. Missing records are skipped silently so a save never fails because of a deleted record.
- Record page ("Mentioned in"): each row shows the note title, kind (note or article) and, for other cases' notes, the case code. With sharing off and no `?case=` context, nothing is listed and the page says mentions from other case files are hidden.
- Deleting a record removes its mention links (links to parties have no foreign key, so delete code removes them by hand, as for sources). Mentions do **not** block deleting a record; existing chips keep their last label as plain, non-clickable text. *(Assistant's call.)*
- Opening a note refreshes party chips to the records' current names; a chip whose record no longer exists shows its last label unlinked.

## Constraints

- No new database migration is expected: the `links` table already holds any `dst_type`. If the builder finds one is needed, it is migration 10 (current is 9), appended and never editing an existing entry.
- Every new `#[tauri::command]` must be registered in `invoke_handler`. Logic stays in plain functions taking `&Connection`.
- Validate in Rust. The visibility rule is enforced in Rust (not only hidden in the UI), and error messages never name another case.
- Exports stay publish-safe: only http(s)/mailto links survive; internal chips are plain text.
- Invalidate the matching query keys in `src/lib/queries.ts` when a mutation changes data shown elsewhere.
- The `[[` picker keeps its current keyboard behaviour (arrow keys, Enter, Escape).

## Non-goals (this slice)

- Creating a person or company from inside the `[[` picker (create them in the directory or the Connections picker first).
- Adding mentioned people to the graph automatically, or showing mention counts on graph nodes.
- A different trigger character such as `@`.
- A "cited in notes" list on source records (separate known gap).
- Merging duplicate records.

## Anti-goals (a failure even if it technically works)

- The "Mentioned in" list revealing another case's notes, titles or codes for a record that is not shared.
- Exports or search results showing `party:<id>` or other internal ids.
- A note failing to save, or a stale chip breaking the editor, because a mentioned record was renamed or deleted.
- Breaking existing note and source links, backlinks, or the existing `[[` create-note flow.

## Calls made by the assistant (not yet confirmed by the user)

- **Who sees which mentions.** The command takes `(party_id, case_id: Option<i64>)`. Notes in `case_id` are always listed. Notes in other cases are listed only if the record is shared. With no `case_id` and an unshared record, nothing is listed and the page says mentions from case files are hidden. `?case=` comes from the URL and is not a security boundary, which matches the earlier "UI privacy feature, not access control" stance.
- **Rows.** A row shows title and kind. The case code shows only for notes outside the current `case_id` (or for every row when shared and no `case_id`).
- **Picker search.** Records come from the existing global record search built in the previous slice (`search_parties`), not from the FTS `search` table, so no migration and no new kind code are needed.
- **Name refresh.** Party chips are refreshed to current names when a note is opened, as a change that does not mark the note dirty or trigger autosave. The stored JSON updates at the next normal save. A rename made while a note is open elsewhere appears on reopen. Exports resolve current names from the database at export time.
- **Search text.** A note's searchable text may keep the old name until the note is saved again. Accepted.
- **Existing backlinks.** Party rows in `links` must not appear in a note's own backlinks, which query by `dst_type = 'document'`. The builder confirms this with a test.

## Left to the builder

- How chip labels are refreshed on open (editor transaction, or resolved when loading the document) and how exports obtain current names (resolved at export time from the database is acceptable).
- The picker's grouping layout, chip styling, and the exact row cap.
- The shape of the backend command that lists mentions for a record (it must take the optional case context and apply the visibility rule), and the test plan. At minimum: a mention appears on the record page; another case's mention is hidden for an unshared record and shown for a shared one; deleting a record removes its mention links; saving a note that mentions a missing record succeeds; renaming a record is reflected on open and in exports; exports contain no `party:` text.
