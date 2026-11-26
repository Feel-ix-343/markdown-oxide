# KB customizations (branch `kb-customizations`)

Local changes on top of upstream `main` (`6a33165`, version 0.25.12) that port the behaviour
the knowledge-base adapter (`~/knowledge/tools/markdown-oxide`) adds around the pinned upstream
binary. Uncommitted; for review.

| Change | Files | Behaviour |
|---|---|---|
| Relative Markdown links | `src/vault/mod.rs` | `[x](../dir/note.md)`, `[x](./note.md)`, `[x](guide/note.md)` and `[[../dir/note]]` resolve relative to the linking file: definition, references/backlinks, and no false "Unresolved" diagnostic. Plain wiki links keep vault-root/basename semantics; `/abs` and paths escaping the vault are not resolved. |
| GitHub heading anchors | `src/vault/mod.rs` | `#2d5-lightweight-shell-commands--stock-tmux` matches `### 2d.5. Lightweight shell commands — stock tmux` (github-slugger rules without `-1` duplicate suffixes). Moxide's own forms (`#Some Heading`, `#Some-Heading`) still work. |
| Standalone `<a id>` anchors | `src/vault/mod.rs` | A line containing only `<a id="x"></a>` (or `name=`), outside code blocks, is a valid `note#x` target. Inline, in-heading and other-tag IDs are ignored. |
| Clearer diagnostics | `src/diagnostics.rs`, `src/config.rs` | "Unresolved Reference" keeps its prefix and appends a reason for placeholders/patterns (`*`, `...`, `XX`, `<…>`) and for targets that exist on disk but outside the indexed vault. New opt-in `ambiguous_diagnostics = true` warns when a link matches several notes. |
| Position encoding | `src/main.rs` | Announces `utf-32` (code points, which the server already computes) when the client offers it; otherwise unchanged. Fixes column offsets after emoji for such clients. |

Tests: 7 new (`kb_customization_tests` in each file), 84/84 total pass.

Not ported, deliberately: the adapter's private snapshot, before/after inventory check, binary
hash pin, result bounds and line limit (Moxide already caps files at 10,000 lines). These are
client-side safety properties, not editor features.

Lab check against a copy of the KB (40 notes, same queries, through the existing adapter):
backlinks 529 -> 559 (missing vs the audit's graph 31 -> 1), unresolved diagnostics 70 -> 3,
outlines identical. The remaining miss and the 3 diagnostics are genuine KB link defects.
