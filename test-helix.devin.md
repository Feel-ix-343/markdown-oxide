# Test Markdown-Oxide in Helix

## Outcome

Verify that markdown-oxide LSP features work correctly in Helix, including wiki link completions, tag completions, and fuzzy matching against the `TestFiles/` directory.

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

Start a screen recording and test each feature:

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

### 6. Clean up

Undo any test edits: `Escape`, then `u` repeatedly.
Quit without saving: `:q!`

## Specifications

- Wiki link completions must show files, headings, and block references with fuzzy matching
- Tag completions must show hierarchical tags (including nested tags like `tag/subtag`)
- The LSP server process (`markdown-oxide`) must be running (verify with `pgrep`)

## Advice

- Helix has built-in language server support for markdown-oxide -- no configuration files are needed. Just ensure the `markdown-oxide` binary is on PATH.
- Block completions (unindexed blocks via `[[ `) are NOT supported in Helix yet (as noted in the Features Index docs).
- Helix's health check (`hx --health markdown`) is useful for verifying the LSP binary is detected.
- Helix may also show `marksman` as a configured language server for markdown. This is fine -- markdown-oxide takes priority if both are available.
- Use `:log-open` inside Helix to view the editor log for debugging LSP issues.

## Forbidden Actions

- Do not modify the TestFiles content permanently (undo all test edits)
- Do not force push or modify the main branch
