# Rule files

One file per numbered or lettered constraint, `<id>.md`, each the single
authority for its rule's text. Read rules on their topic page
(`docs/agents/<topic>.md#constraint-<id>`), which
`scripts/gen_rule_docs.py` renders from these files; the
[router](../README.md)'s stable rule index is generated from their front
matter. A rule's text is rendered into its topic page, so its relative links
resolve from `docs/agents/`, not from this directory.

Front matter: `rule` (the id, equal to the file name), `topic` (the page stem)
and `summary` (the router's navigation-only opening sentence; omitted for a
lead-in with no router row, such as `11-series`). How a card adds a rule:
[operations](../operations.md#card-briefs-and-delegation).
