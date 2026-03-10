---
name: testing-zed
description: Test markdown-oxide LSP features in Zed
---

# Test Markdown-Oxide Extension in Zed

## Overview
markdown-oxide is an LSP server for markdown/Obsidian vaults. The Zed extension source is at https://github.com/Feel-ix-343/markdown-oxide-zed. It first checks PATH for the `markdown-oxide` binary via `worktree.which()`, then falls back to downloading from GitHub releases.

## Outcome

Verify that the markdown-oxide Zed extension works correctly by installing Zed, building the markdown-oxide binary, installing the extension, and testing LSP features (completions, go-to-definition, hover, tags) against the `TestFiles/` directory. Testing is done in two recorded phases: first reproduce/demonstrate the current behavior, then validate the fix or expected behavior.

## Procedure

### 1. Build the markdown-oxide binary

```bash
cd ~/repos/markdown-oxide && cargo build
sudo cp target/debug/markdown-oxide /usr/local/bin/markdown-oxide
```

Verify it's on PATH: `which markdown-oxide`

### 2. Install Zed

```bash
curl -fsSL https://zed.dev/install.sh | sh
```

### 3. Launch Zed with logging enabled

Launch the Zed editor binary directly so you can capture logs:

```bash
RUST_LOG=info /home/ubuntu/.local/zed.app/libexec/zed-editor ~/repos/markdown-oxide/TestFiles > /tmp/zed.log 2>&1 &
```

Wait a few seconds for Zed to start, then check the screenshot to confirm it opened.

### 4. Install the Markdown-Oxide extension

1. Open Extensions panel: `Ctrl+Shift+X`
2. Search for `markdown-oxide`
3. Click **Install**
4. Wait for the install to complete (button changes to "Uninstall")

### 5. Trust the worktree (CRITICAL)

Zed opens projects in **Restricted Mode** which blocks language servers from starting. You MUST trust the project:

1. Look for a "Restricted Mode" indicator near the top of the Zed window
2. Click on it
3. Click **Trust and Continue**

Without this, the LSP will NOT start. You can confirm the issue by checking `/tmp/zed.log` for:
```
Waiting for worktree "..." to be trusted, before starting language server markdown-oxide
```

After trusting, the log should show:
```
starting language server process. binary path: "/usr/local/bin/markdown-oxide"
```

Verify the LSP process is running: `pgrep -a markdown-oxide`

### 6. Test LSP features

Open `TestFiles/Test.md`. Testing is split into two recorded phases:

#### Phase 1: Reproduce current behavior

Start a screen recording (`recording_start`). Demonstrate the current state of each feature **before any fix**. This establishes a baseline and captures any issues:

- Annotate the recording: "Phase 1: Reproducing current behavior in Zed"
- Exercise each feature below and note what works and what doesn't
- Stop the recording (`recording_stop`) when done

#### Phase 2: Validate the fix

After applying the fix (rebuild markdown-oxide, copy to PATH, restart LSP in Zed via `Ctrl+Shift+P` then `editor: restart language server`):

- Start a new screen recording (`recording_start`)
- Annotate the recording: "Phase 2: Validating fix in Zed"
- Re-test each feature and confirm it works correctly
- Stop the recording (`recording_stop`) when done

Test each feature:

#### Wiki Link Completions
- Go to the end of the file, add a new line, and type `[[`
- A completion menu should appear showing files, headings, and blocks
- Type to fuzzy-filter (e.g., `[[Reso` should show "Resolved File")
- A preview panel should appear on the right with file content and backlinks
- Press Escape and undo changes when done

#### Go-to-Definition
- Place cursor inside `[[This is another link]]` (line 31)
- Press `F12`
- Should navigate to `This is another link.md`
- Go back with the back navigation button or `Ctrl+-`

#### Hover
- Hover the mouse over `![[This is another link#^9d273|test block link]]` (line 24)
- Wait ~1-2 seconds for the hover popup
- Should show: **Block Preview** (content of the referenced block) and **Backlinks** (all files referencing this entity)

#### Tag Completions
- Add a new line and type `#ta`
- Should show hierarchical tag completions: `tag`, `tag/subtag`, `tag/othersubtag`, `mapofcontent/tag`, etc.
- Preview panel should show backlinks for the selected tag
- Press Escape and undo changes when done

#### References (optional)
- Place cursor on a heading like `# Heading 1`
- Press `Shift+F12` to find all references/backlinks

### 7. Post recordings to PR

After both recording phases are complete, post the recordings as comments on the PR:

1. Use `git_comment_on_pr` to post the Phase 1 recording with a comment like:
   > **Phase 1: Reproducing current behavior in Zed**
   > ![Phase 1 recording](/path/to/phase1-recording.mp4)

2. Use `git_comment_on_pr` to post the Phase 2 recording with a comment like:
   > **Phase 2: Validating fix in Zed**
   > ![Phase 2 recording](/path/to/phase2-recording.mp4)

This provides reviewers with visual evidence of the issue and its resolution.

### 8. Clean up

Undo any test edits in the files. Report results to the user.

## Available Test Files

- `TestFiles/Test.md` -- Main test file with headings, wiki links, block refs, tags
- `TestFiles/Resolved File.md` -- Has `# Resolved Heading` and heading links
- `TestFiles/Another Test.md` -- Has `# This is a test heading` and `## This is a nested test heading`
- `TestFiles/This is another link.md` -- Target for wiki link navigation tests

## Specifications

- All four core features (wiki link completions, go-to-definition, hover, tag completions) must work
- The LSP server process (`markdown-oxide`) must be running (verify with `pgrep`)
- Completions should include a preview panel showing content and backlinks
- Go-to-definition should navigate to the correct target file
- Two screen recordings must be produced: one showing current behavior (reproduce), one showing the fix (validate)
- Both recordings must be posted as comments on the PR

## Advice

- The Zed GPU warning dialog is normal on headless/VM environments using software rendering (llvmpipe). Just dismiss it.
- If the LSP doesn't start, always check the worktree trust status first -- this is the #1 issue.
- The Zed extension source is at https://github.com/Feel-ix-343/markdown-oxide-zed. It first checks PATH for the `markdown-oxide` binary via `worktree.which()`, then falls back to downloading from GitHub releases.
- Use `Ctrl+Shift+P` then `dev: open language server logs` to inspect LSP communication inside Zed.
- Use `Ctrl+Shift+P` then `editor: restart language server` if you need to restart the LSP.
- Zed's CLI wrapper (`zed`) just signals the running editor process -- to capture logs, launch the binary directly at `~/.local/zed.app/libexec/zed-editor`.
- After rebuilding the binary, restart the LSP via command palette rather than relaunching Zed.
- `cargo build` (debug) is much faster than `cargo build --release` -- use debug for testing iterations.

## Forbidden Actions

- Do not modify the TestFiles content permanently (undo all test edits)
- Do not force push or modify the main branch
