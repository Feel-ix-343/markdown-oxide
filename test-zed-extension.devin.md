# Test Markdown-Oxide Extension in Zed

## Outcome

Verify that the markdown-oxide Zed extension works correctly by installing Zed, building the markdown-oxide binary, installing the extension, and testing LSP features (completions, go-to-definition, hover, tags) against the `TestFiles/` directory.

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

Open `TestFiles/Test.md` and start a screen recording. Test each feature:

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

### 7. Clean up

Undo any test edits in the files. Stop the recording and report results.

## Specifications

- All four core features (wiki link completions, go-to-definition, hover, tag completions) must work
- The LSP server process (`markdown-oxide`) must be running (verify with `pgrep`)
- Completions should include a preview panel showing content and backlinks
- Go-to-definition should navigate to the correct target file

## Advice

- The Zed GPU warning dialog is normal on headless/VM environments using software rendering (llvmpipe). Just dismiss it.
- If the LSP doesn't start, always check the worktree trust status first -- this is the #1 issue.
- The Zed extension source is at https://github.com/Feel-ix-343/markdown-oxide-zed. It first checks PATH for the `markdown-oxide` binary via `worktree.which()`, then falls back to downloading from GitHub releases.
- Use `Ctrl+Shift+P` then `dev: open language server logs` to inspect LSP communication inside Zed.
- Use `Ctrl+Shift+P` then `editor: restart language server` if you need to restart the LSP.
- Zed's CLI wrapper (`zed`) just signals the running editor process -- to capture logs, launch the binary directly at `~/.local/zed.app/libexec/zed-editor`.

## Forbidden Actions

- Do not modify the TestFiles content permanently (undo all test edits)
- Do not force push or modify the main branch
