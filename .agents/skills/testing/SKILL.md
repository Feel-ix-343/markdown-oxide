---
name: testing
description: Meta-skill for testing markdown-oxide LSP features. Routes to the appropriate editor-specific testing skill (Neovim, Helix, or Zed).
---

# Test Markdown-Oxide

This is a meta-skill that routes to the appropriate editor-specific testing skill for markdown-oxide.

## Which skill to use

Choose the testing skill based on the editor you want to test in:

| Editor | Skill | When to use |
|--------|-------|-------------|
| Neovim | `testing-neovim` | Default choice. Most configurable, supports all features including block linking. Requires Neovim v0.11+. |
| Helix | `testing-helix` | Zero-config setup. Built-in LSP support, but does NOT support block linking. |
| Zed | `testing-zed` | Extension-based. Requires worktree trust step. Good for GUI-based testing. |

## Procedure

1. If the user specifies an editor, invoke the corresponding skill above.
2. If no editor is specified, default to `testing-neovim` (most complete feature coverage).
3. Follow all steps in the invoked skill as a strict checklist.

## Quick reference

To invoke a specific editor skill directly, use:
- `@skills:testing-neovim`
- `@skills:testing-helix`
- `@skills:testing-zed`
