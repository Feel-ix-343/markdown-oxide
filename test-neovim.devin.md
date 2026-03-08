# Test Markdown-Oxide in Neovim

## Outcome

Verify that markdown-oxide LSP features work correctly in Neovim, including wiki link completions, block linking (with block ID insertion via `:wall`), go-to-definition, hover with backlinks, and tag completions against the `TestFiles/` directory.

## Procedure

### 1. Build the markdown-oxide binary

```bash
cd ~/repos/markdown-oxide && cargo build
sudo cp target/debug/markdown-oxide /usr/local/bin/markdown-oxide
```

Verify it's on PATH: `which markdown-oxide`

### 2. Install Neovim (if not installed)

```bash
curl -fsSL -o /tmp/nvim.appimage https://github.com/neovim/neovim/releases/latest/download/nvim-linux-x86_64.appimage
chmod +x /tmp/nvim.appimage
cd /tmp && /tmp/nvim.appimage --appimage-extract
sudo mv /tmp/squashfs-root /opt/nvim
sudo ln -sf /opt/nvim/usr/bin/nvim /usr/local/bin/nvim
```

Verify: `nvim --version | head -1` (should be v0.11+)

### 3. Configure Neovim for markdown-oxide

Create `~/.config/nvim/init.lua`:

```lua
-- Minimal Neovim config for testing markdown-oxide LSP
vim.opt.number = true
vim.opt.signcolumn = "yes"
vim.opt.completeopt = { "menu", "menuone", "noselect" }

vim.lsp.config('markdown_oxide', {
  cmd = { 'markdown-oxide' },
  filetypes = { 'markdown' },
  root_markers = { '.obsidian', '.moxide.toml', '.git' },
  capabilities = {
    workspace = {
      didChangeWatchedFiles = {
        dynamicRegistration = true,
      },
    },
  },
})

vim.lsp.enable('markdown_oxide')

vim.api.nvim_create_autocmd('LspAttach', {
  callback = function(args)
    local opts = { buffer = args.buf }
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
    vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
    vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)
    vim.keymap.set('i', '<C-Space>', function()
      vim.lsp.completion.trigger()
    end, opts)
    vim.lsp.completion.enable(true, args.data.client_id, args.buf, { autotrigger = true })

    local client = vim.lsp.get_client_by_id(args.data.client_id)
    if client and client.name == "markdown_oxide" then
      vim.api.nvim_create_user_command("Daily", function(cmd_args)
        vim.lsp.buf.execute_command({ command = "jump", arguments = { cmd_args.args } })
      end, { desc = "Open daily note", nargs = "*" })
    end
  end,
})
```

This uses Neovim 0.11+ built-in LSP support (`vim.lsp.config` / `vim.lsp.enable`). The `dynamicRegistration = true` setting is critical for block linking to work.

### 4. Launch Neovim

Open a terminal emulator (e.g., `konsole`) and launch Neovim on TestFiles:

```bash
konsole --workdir ~/repos/markdown-oxide/TestFiles -e bash -c "nvim Test.md" &
```

Wait for Neovim to open and the LSP to attach. Verify with: `pgrep -a markdown-oxide`

### 5. Test LSP features

Start a screen recording and test each feature:

#### Wiki Link Completions
- In Normal mode, press `G` to go to end of file, then `o` to open a new line
- Type `[[` -- a completion menu should appear with files, headings, and blocks
- Type to fuzzy-filter (e.g., `[[Reso` should show "Resolved File")
- Press `Escape` and `u` to undo when done

#### Block Linking (CRITICAL Neovim-specific feature)
- Press `o` to open a new line in insert mode
- Type `[[ ` (two brackets then a space) -- this triggers the unindexed block completer
- A list of text blocks from across the vault appears
- Type to fuzzy-filter (e.g., `test file with some`)
- Use `Ctrl+n`/`Ctrl+p` to navigate, `Ctrl+y` to accept
- After accepting, a link like `[[Another Test 2#^f311g|text]]` is inserted with a generated block ID
- **You MUST run `:wall` to write all buffers** -- the block ID is inserted into the target file as an unsaved buffer edit
- Verify the block ID was inserted: check the target file for the `^blockid` suffix

#### Go-to-Definition
- Navigate to a line with `[[This is another link]]` (around line 31)
- Position cursor inside the link text (e.g., `fT` to find the `T`)
- Press `gd` -- should navigate to `This is another link.md`
- Press `Ctrl+o` to go back

#### Hover
- With cursor on a wiki link like `[[This is another link]]`
- Press `K` (Shift+k) -- a hover popup should show:
  - **File Preview**: contents of the linked file
  - **Backlinks**: all files referencing this entity
- Press `Escape` or any key to dismiss

#### Tag Completions
- Press `o` to open a new line, type `#ta`
- Should show hierarchical tags: `tag`, `tag/subtag`, `tag/othersubtag`, `mapofcontent/tag`, etc.
- Press `Escape` and `u` to undo when done

#### References (optional)
- Place cursor on a heading like `# Heading 1`
- Press `gr` to find all references/backlinks

### 6. Clean up

Undo any test edits: `Escape`, then `u` repeatedly until "Already at oldest change".
Quit without saving: `:qa!`

## Specifications

- Wiki link completions must show files, headings, and block references
- Block linking must insert a `^blockid` into the target file after `:wall`
- Go-to-definition must navigate to the correct target file
- Hover must show file preview and backlinks
- Tag completions must show hierarchical tags
- The LSP server process (`markdown-oxide`) must be running (verify with `pgrep`)

## Advice

- Neovim 0.11+ is required for `vim.lsp.config` / `vim.lsp.enable`. Older versions need `nvim-lspconfig` plugin.
- The `dynamicRegistration = true` capability is essential for block linking and the "Create Unresolved File" code action to work.
- On headless/VM environments, the AppImage may fail with FUSE errors. Use `--appimage-extract` to extract without FUSE.
- Block completions are triggered by `[[ ` (with a space after `[[`). Without the space, you get regular file/heading completions.
- After accepting a block completion, the block ID is edited into the target file's buffer but NOT saved. You must run `:wall` to persist it.
- For a richer completion UI, install `nvim-cmp` with `cmp-nvim-lsp`. The built-in `vim.lsp.completion` works but `nvim-cmp` provides better UX.
- Use `:LspLog` to inspect LSP communication for debugging.

## Forbidden Actions

- Do not modify the TestFiles content permanently (undo all test edits)
- Do not force push or modify the main branch
