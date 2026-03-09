# Test Markdown-Oxide in Helix

## Overview
markdown-oxide is an LSP server for markdown/Obsidian vaults. Helix has built-in language server support for markdown-oxide -- no configuration files are needed. Just ensure the `markdown-oxide` binary is on PATH.

## Outcome

Verify that markdown-oxide LSP features work correctly in Helix, including wiki link completions, tag completions, and fuzzy matching against the `TestFiles/` directory. Testing is done in two recorded phases: first reproduce/demonstrate the current behavior, then validate the fix or expected behavior.

## Procedure

### 1. Build the markdown-oxide binary

```bash
cd ~/repos/markdown-oxide && cargo build
sudo cp target/debug/markdown-oxide /usr/local/bin/markdown-oxide
```

Verify it's on PATH: `which markdown-oxide`

### 2. Install Helix (if not installed)

```bash
HELIX_VERSION=$(curl -s "https://api.github.com/repos/helix-editor/helix/releases/latest" | grep -Po '"tag_name": "\K[0-9.]+')
wget -qO /tmp/helix.tar.xz "https://github.com/helix-editor/helix/releases/latest/download/helix-${HELIX_VERSION}-x86_64-linux.tar.xz"
sudo mkdir -p /opt/helix
sudo tar xf /tmp/helix.tar.xz --strip-components=1 -C /opt/helix
sudo ln -sf /opt/helix/hx /usr/local/bin/hx
```

Verify: `hx --version`

### 3. Verify Helix recognizes markdown-oxide

```bash
hx --health markdown
```

You should see: `markdown-oxide: /usr/local/bin/markdown-oxide` (with a checkmark). Helix has built-in support for markdown-oxide -- no additional configuration is needed.

### 4. Launch Helix

Open a terminal emulator and launch Helix on TestFiles:

```bash
konsole --workdir ~/repos/markdown-oxide/TestFiles -e bash -c "hx Test.md" &
```

Wait for Helix to open. Verify the LSP is running: `pgrep -a markdown-oxide`

### 5. Test LSP features

Testing is split into two recorded phases:

#### Phase 1: Reproduce current behavior

Start a screen recording (`recording_start`). Demonstrate the current state of each feature **before any fix**. This establishes a baseline and captures any issues:

- Annotate the recording: "Phase 1: Reproducing current behavior in Helix"
- Exercise each feature below and note what works and what doesn't
- Stop the recording (`recording_stop`) when done

#### Phase 2: Validate the fix

After applying the fix (rebuild markdown-oxide, copy to PATH, quit and relaunch Helix to restart the LSP):

- Start a new screen recording (`recording_start`)
- Annotate the recording: "Phase 2: Validating fix in Helix"
- Re-test each feature and confirm it works correctly
- Stop the recording (`recording_stop`) when done

Test each feature:

#### Wiki Link Completions
- In Normal mode, press `g` then `e` to go to end of file
- Press `o` to open a new line below (enters Insert mode)
- Type `[[` -- a completion menu should appear with files, headings, and block references
- Type to fuzzy-filter (e.g., `[[Reso` should show "Resolved File" and "Resolved File#Resolved Heading")
- Press `Escape` and `u` to undo when done

#### Tag Completions
- Press `o` to open a new line, type `#ta`
- Should show hierarchical tag completions: `tag`, `tag/subtag`, `tag/othersubtag`, `mapofcontent/tag`, `mapofcontent/tag/supertag`, `mapofcontent/tag/supertag/tag`
- All labeled as "keyword" type
- Press `Escape` and `u` to undo when done

#### Go-to-Definition (optional)
- Navigate to a line with `[[This is another link]]`
- Position cursor inside the link
- Press `g` then `d` for go-to-definition

#### Hover (optional)
- Position cursor on a wiki link
- Press `Space` then `k` to show hover info

### 6. Post recordings to PR

After both recording phases are complete, post the recordings as comments on the PR:

1. Use `git_comment_on_pr` to post the Phase 1 recording with a comment like:
   > **Phase 1: Reproducing current behavior in Helix**
   > ![Phase 1 recording](/path/to/phase1-recording.mp4)

2. Use `git_comment_on_pr` to post the Phase 2 recording with a comment like:
   > **Phase 2: Validating fix in Helix**
   > ![Phase 2 recording](/path/to/phase2-recording.mp4)

This provides reviewers with visual evidence of the issue and its resolution.

### 7. Clean up

Undo any test edits: `Escape`, then `u` repeatedly.
Quit without saving: `:q!`

## Available Test Files

- `TestFiles/Test.md` -- Main test file with headings, wiki links, block refs, tags
- `TestFiles/Resolved File.md` -- Has `# Resolved Heading` and heading links
- `TestFiles/Another Test.md` -- Has `# This is a test heading` and `## This is a nested test heading`
- `TestFiles/This is another link.md` -- Target for wiki link navigation tests

## Specifications

- Wiki link completions must show files, headings, and block references with fuzzy matching
- Tag completions must show hierarchical tags (including nested tags like `tag/subtag`)
- The LSP server process (`markdown-oxide`) must be running (verify with `pgrep`)
- Two screen recordings must be produced: one showing current behavior (reproduce), one showing the fix (validate)
- Both recordings must be posted as comments on the PR

## Advice

- Helix has built-in language server support for markdown-oxide -- no configuration files are needed. Just ensure the `markdown-oxide` binary is on PATH.
- Block completions (unindexed blocks via `[[ `) are NOT supported in Helix yet (as noted in the Features Index docs).
- Helix's health check (`hx --health markdown`) is useful for verifying the LSP binary is detected.
- Helix may also show `marksman` as a configured language server for markdown. This is fine -- markdown-oxide takes priority if both are available.
- Use `:log-open` inside Helix to view the editor log for debugging LSP issues.
- After rebuilding the binary, you must quit Helix and relaunch to restart the LSP.
- `cargo build` (debug) is much faster than `cargo build --release` -- use debug for testing iterations.

## Forbidden Actions

- Do not modify the TestFiles content permanently (undo all test edits)
- Do not force push or modify the main branch
