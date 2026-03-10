---
name: testing
description: Test markdown-oxide LSP changes by reproducing the issue and validating the fix in an editor (Neovim, Helix, or Zed).
---

# Test Markdown-Oxide

Test markdown-oxide changes using a two-phase approach: first reproduce the issue, then validate the fix. Each phase is a screen recording posted to the PR.

## Procedure

### 1. Choose an editor

Pick the editor-specific skill based on the context, or default to Neovim:

| Editor | Skill | Notes |
|--------|-------|-------|
| Neovim | `testing-neovim` | Default. Most complete feature coverage, including block linking. |
| Helix | `testing-helix` | Zero-config. No block linking support. |
| Zed | `testing-zed` | GUI-based. Requires worktree trust step. |

### 2. Phase 1: Reproduce the issue

Build the **current** markdown-oxide binary (before your fix) and follow the chosen editor skill to:

1. Start a screen recording
2. Exercise the relevant LSP features to demonstrate the bug or current behavior
3. Stop the recording and post it to the PR

### 3. Apply the fix

Rebuild markdown-oxide with your changes, copy the new binary to PATH, and restart the editor/LSP.

### 4. Phase 2: Validate the fix

Follow the same editor skill again to:

1. Start a new screen recording
2. Re-test the same features and confirm the fix works
3. Stop the recording and post it to the PR

## Quick reference

Invoke a specific editor skill directly with:
- `@skills:testing-neovim`
- `@skills:testing-helix`
- `@skills:testing-zed`
