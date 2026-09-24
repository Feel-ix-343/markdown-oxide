---
name: release-testing
description: Full release validation for markdown-oxide. Builds the latest binary and tests all implemented features across Neovim, Helix, and Zed in parallel subsessions. Run this before cutting a release.
---

# Release Testing

Validate all implemented markdown-oxide features across every supported editor before a release. This skill spawns three parallel Devin subsessions — one for each editor — so all testing happens concurrently.

## When to Use

Run this skill before triggering the `release.yml` workflow. A release must not go out without passing this gate.

## Procedure

### 1. Build the release candidate binary

```bash
cd ~/repos/markdown-oxide && git pull && git submodule update --init --recursive
cargo build
sudo cp target/debug/markdown-oxide /usr/local/bin/markdown-oxide
which markdown-oxide
```

### 2. Spawn three parallel subsessions

Create three Devin subsessions, one for each editor. Each subsession receives the same feature checklist (Section 4 below) and follows its editor-specific skill for setup and interaction. All three run concurrently.

| Subsession | Editor Skill | Notes |
|------------|-------------|-------|
| 1 | `@skills:testing-neovim` | Full feature coverage including block linking |
| 2 | `@skills:testing-helix` | No block linking support; skip that item |
| 3 | `@skills:testing-zed` | No rename support; skip rename items |

Each subsession prompt should be:

> Build markdown-oxide from the latest code on the current branch, install the binary, and run a full release validation in **{editor}** following `@skills:testing-{editor}`.
>
> **Skip Phase 1** (reproduce). There is no specific bug to reproduce — this is a release validation, not a bug fix. Only run **Phase 2** (validate).
>
> Test every feature in the checklist below. For each feature, record PASS or FAIL with a brief note. Capture the entire session as a single screen recording.
>
> **Feature Checklist:**
>
> **Completions**
> - [ ] Wikilink completions to files and headings — type `[[` and verify files, headings appear
> - [ ] Markdown link completions to files and headings — type `[](` and verify completions
> - [ ] Block completions — type `[[ ` (with space) and verify block-level completions *(Neovim only)*
> - [ ] Tag completions — type `#ta` and verify hierarchical tags appear (`tag`, `tag/subtag`, `tag/othersubtag`, `mapofcontent/tag`, `mapofcontent/tag/supertag`, `mapofcontent/tag/supertag/tag`)
> - [ ] Footnote completions — type `[^` and verify footnote completions from the active file
> - [ ] Unresolved file and heading completions — type `[[nonexistent` and verify unresolved items appear
> - [ ] Callout completions — type `> [!` and verify callout types appear
> - [ ] Nested callout completions — inside an existing callout, type `> [!` and verify nested callout completions
> - [ ] Alias completions — verify that file aliases defined in frontmatter appear in `[[` completions
> - [ ] Daily note completions — type `[[Daily` or the equivalent and verify natural language date completions (e.g., `next tuesday`, `tomorrow`, `two days ago`)
>
> **References**
> - [ ] File references — place cursor on a file link, invoke references, verify all references to that file are listed
> - [ ] Heading references — place cursor on a heading, invoke references, verify references to that heading
> - [ ] Indexed block references — place cursor on a block ID, invoke references, verify references
> - [ ] Tag references — place cursor on a tag, invoke references, verify references to the tag and subtags
> - [ ] Footnote references — place cursor on a footnote, invoke references, verify all uses in the file
> - [ ] Unresolved file and heading references — place cursor on an unresolved link, invoke references
>
> **Hover**
> - [ ] Hover on a file link — verify preview text and backlinks snapshot appear
> - [ ] Hover on a heading link — verify heading content and backlinks appear
>
> **Code Actions**
> - [ ] Create file for unresolved file link — place cursor on `[[nonexistent]]`, invoke code action, verify file is created
> - [ ] Append heading to file — invoke code action to append a heading, verify it is added
>
> **Diagnostics**
> - [ ] Unresolved reference diagnostic — verify that links to nonexistent files show a diagnostic warning
>
> **Symbols**
> - [ ] File symbols (document outline) — invoke document symbols, verify hierarchical heading outline
> - [ ] Workspace symbols — invoke workspace symbols, search for a file name, heading, or tag, verify results
>
> **Rename** *(Neovim and Helix only — not supported in Zed)*
> - [ ] Rename file — rename a file and verify all references are updated
> - [ ] Rename heading — rename a heading and verify all references are updated
> - [ ] Rename tag — rename a tag and verify all references are updated
>
> **Daily Notes**
> - [ ] Open daily note via command — run `:Daily today` (Neovim) or equivalent and verify the daily note opens or is created
>
> After testing, post the screen recording and a summary table of PASS/FAIL results. Undo all test edits and do not modify TestFiles permanently.

### 3. Collect results

Wait for all three subsessions to complete. Each will post:
- A screen recording of the full validation
- A PASS/FAIL summary table

### 4. Evaluate go/no-go

Review the three summary tables. The release is **blocked** if any implemented feature shows FAIL in any editor where it is supported. Summarize the results in a single comment:

```
## Release Testing Results

| Feature | Neovim | Helix | Zed |
|---------|--------|-------|-----|
| Wikilink completions | PASS | PASS | PASS |
| ... | ... | ... | ... |

**Verdict: GO / NO-GO**
```

## Feature Coverage by Editor

This table documents which features are supported in which editor. Subsessions should skip features marked N/A.

| Feature | Neovim | Helix | Zed |
|---------|--------|-------|-----|
| Wikilink completions | Yes | Yes | Yes |
| Markdown link completions | Yes | Yes | Yes |
| Block completions | Yes | N/A | N/A |
| Tag completions | Yes | Yes | Yes |
| Footnote completions | Yes | Yes | Yes |
| Unresolved completions | Yes | Yes | Yes |
| Callout completions | Yes | Yes | Yes |
| Nested callout completions | Yes | Yes | Yes |
| Alias completions | Yes | Yes | Yes |
| Daily note completions | Yes | Yes | Yes |
| File references | Yes | Yes | Yes |
| Heading references | Yes | Yes | Yes |
| Indexed block references | Yes | Yes | Yes |
| Tag references | Yes | Yes | Yes |
| Footnote references | Yes | Yes | Yes |
| Unresolved references | Yes | Yes | Yes |
| Hover (file link) | Yes | Yes | Yes |
| Hover (heading link) | Yes | Yes | Yes |
| Code action: create file | Yes | Yes | Yes |
| Code action: append heading | Yes | Yes | Yes |
| Diagnostics: unresolved ref | Yes | Yes | Yes |
| File symbols | Yes | Yes | Yes |
| Workspace symbols | Yes | Yes | Yes |
| Rename file | Yes | Yes | N/A |
| Rename heading | Yes | Yes | N/A |
| Rename tag | Yes | Yes | N/A |
| Daily note open | Yes | Yes | Yes |

## Forbidden Actions

- Do not modify TestFiles content permanently (undo all test edits)
- Do not push to main or create tags — this skill is a pre-release gate only
- Do not skip features that are supported in the editor being tested
