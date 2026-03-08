# Testing markdown-oxide LSP in Neovim

## Overview
markdown-oxide is an LSP server for markdown/Obsidian vaults. It cannot be tested via browser or direct CLI invocation — it requires an editor with LSP support. Neovim v0.11+ is the recommended testing environment.

## Prerequisites
- Rust toolchain (cargo) for building the binary
- Neovim v0.11+ (for `vim.lsp.config` / `vim.lsp.enable` built-in LSP support)
- The `TestFiles/` directory in the repo root serves as a test vault (has `.obsidian` marker)

## Setup Steps

### 1. Build the binary
```bash
cd ~/repos/markdown-oxide && cargo build
sudo cp target/debug/markdown-oxide /usr/local/bin/markdown-oxide
```
For release builds (slower but optimized): `cargo build --release` then copy from `target/release/`.

### 2. Install Neovim (if not available)
```bash
curl -fsSL -o /tmp/nvim.appimage https://github.com/neovim/neovim/releases/latest/download/nvim-linux-x86_64.appimage
chmod +x /tmp/nvim.appimage
cd /tmp && /tmp/nvim.appimage --appimage-extract
sudo mv /tmp/squashfs-root /opt/nvim
sudo ln -sf /opt/nvim/usr/bin/nvim /usr/local/bin/nvim
```
On headless/VM environments, the AppImage may fail with FUSE errors. Use `--appimage-extract` to extract without FUSE.

### 3. Configure Neovim
Create `~/.config/nvim/init.lua` with the minimal LSP config from `test-neovim.devin.md` in the repo root. Key points:
- Uses `vim.lsp.config` / `vim.lsp.enable` (Neovim 0.11+ only)
- `dynamicRegistration = true` is critical for block linking features
- Key mappings: `gd` (go-to-definition), `gr` (references), `K` (hover), `Ctrl+Space` (trigger completion)

### 4. Launch Neovim
```bash
konsole --workdir ~/repos/markdown-oxide/TestFiles -e bash -c "nvim Test.md" &
```
Wait 2-3 seconds for LSP to attach. Verify with: `pgrep -a markdown-oxide`

## Testing Workflow

### Key LSP Features to Test
1. **Go-to-definition (`gd`)**: Position cursor inside a `[[wiki link]]` and press `gd` — should navigate to the target
2. **Completions**: Type `[[` in insert mode — completion menu should appear with files, headings, blocks
3. **Hover (`K`)**: On a wiki link, press `K` — should show file preview and backlinks
4. **Tag completions**: Type `#ta` — should show hierarchical tag completions
5. **Block linking**: Type `[[ ` (with space after `[[`) — triggers unindexed block completer (Neovim-specific)

### Testing Heading Links Specifically
- Heading links with dashes (`[[File#My-Heading]]`) should resolve correctly
- Heading links with spaces (`[[File#My Heading]]`) should also resolve (lenient matching)
- Completions for headings should show dash-separated format (e.g., `File#My-Heading`)
- Test headings in `TestFiles/Test.md`: `# Heading 1`, `## Here is a nested`, `### Here is a nested third`
- Test cross-file heading links using `TestFiles/Resolved File.md` which has `# Resolved Heading`

### Available Test Files
- `TestFiles/Test.md` — Main test file with headings, wiki links, block refs, tags
- `TestFiles/Resolved File.md` — Has `# Resolved Heading` and heading links
- `TestFiles/Another Test.md` — Has `# This is a test heading` and `## This is a nested test heading`
- `TestFiles/This is another link.md` — Target for wiki link navigation tests

### Recording Tests
- Use `recording_start` before testing, `recording_stop` after
- Use `annotate_recording` at key moments (feature name, pass/fail)
- Post recordings to PR via `git_comment_on_pr`

## Cleanup
- Undo all test edits in Neovim: `Escape`, then `u` repeatedly until "Already at oldest change"
- Quit without saving: `:qa!`
- Do NOT modify TestFiles content permanently

## Gotchas
- After rebuilding the binary, you must quit Neovim and relaunch to restart the LSP (or copy binary while LSP is running and it may pick up changes)
- Neovim's built-in completion can be slow to trigger — use `Ctrl+Space` to force trigger
- The `test-neovim.devin.md` file in the repo root has the full Neovim config to use
- `cargo build` (debug) is much faster than `cargo build --release` — use debug for testing iterations
- Block completions require `[[ ` (bracket-bracket-space) to trigger, different from regular `[[` completions
