# kanban-bridge

Trello's board export (Settings -> Menu -> Print and Export -> Export as JSON)
dumps the whole board as one JSON blob: lists, cards, checklists, members,
labels, activity, all mixed together. If you just want to keep working on the
same board as a plain markdown file - for example with Obsidian's Kanban
community plugin - you end up pulling list names and card names out of that
JSON by hand.

`kanban-bridge` does that conversion for you. Point it at a Trello export and
it writes an Obsidian Kanban markdown file with the same lists, the same card
order, and the same descriptions, skipping anything Trello has archived.

## usage

```
kanban-bridge <input.json> <output.md> [--json]
```

Convert a Trello export to a markdown board:

```
$ kanban-bridge trello-export.json board.md
converted trello-export.json -> board.md
  lists converted: 4
  cards converted: 27
  archived cards skipped: 3
```

Same conversion, but with a machine-readable summary for scripting:

```
$ kanban-bridge trello-export.json board.md --json
{"input":"trello-export.json","output":"board.md","lists_converted":4,"cards_converted":27,"closed_lists_skipped":0,"closed_cards_skipped":3,"orphaned_cards_skipped":0,"warnings":[]}
```

The generated `board.md` looks like this:

```
---

kanban-plugin: board

---

## To do

- [ ] Write the JSON parser

## In progress

- [ ] Wire up the CLI
  first pass, no error handling yet
```

Drop that file straight into an Obsidian vault and the Kanban plugin renders
it.

## what it does not do yet

- Only Trello -> Obsidian is implemented right now; the reverse direction is
  on the list.
- Attachments, labels, checklists, and comments are dropped. Only list name,
  card name, and card description survive the conversion.
- The JSON parser is hand-written (no serde, no third-party crates at all) so
  it covers what Trello's export actually contains, not every edge case in
  the JSON spec - escaped surrogate pairs, for instance, are not reassembled.

## building

Standard library only, no dependencies:

```
cargo build --release
```
