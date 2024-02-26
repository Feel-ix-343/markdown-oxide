# Markdown Oxide


Implementing obsidian PKM features (and possibly more) in the form of a language server allows us to use these features in our favorite text editors and reuse other lsp related plugins (like Telescope, outline, refactoring tools, ...)

## Installation

> [!IMPORTANT]
> I'm working on getting this into package distributions. Installation/configuration should be easier soon.


### Arch Linux

If you are using Arch Linux, you can install the latest Git version through the AUR. The package name is `markdown-oxide-git`. You can install it using your preferred AUR helper; for example:

```sh
paru -S markdown-oxide-git
```

### Manual

Clone the repository and then run `cargo build --release`.

## Usage

To use the language server, you need to follow the instructions for your editor of choice below.

### VSCode

Go to [the VSCode extension readme](./vscode-extension/README.md) and run the commands listed

### Neovim

Make sure rust is installed properly and that you are using nvim cmp (I am not sure if it works in other completion engines)

Adjust your neovim config as follows

```lua
local lspconfig = require('lspconfig')
local configs = require("lspconfig.configs")

configs["markdown_oxide"] = {
  default_config = {
    root_dir = lspconfig.util.root_pattern('.git', vim.fn.getcwd()),
    filetypes = {"markdown"},
    cmd = {"markdown-oxide"} -- This needs to be the path to the markdown-oxide binary, either in your PATH or the full absolute path.
  },
  on_attach = on_attach, -- do this only if you have an on_attach function already
}

require("lspconfig").markdown_oxide.setup({
    capabilities = capabilities -- ensure that capabilities.workspace.didChangeWatchedFiles.dynamicRegistration = true
})
```

then adjust your nvim-cmp source settings for the following. Note that this will likely change in the future.

```lua
{
name = 'nvim_lsp',
  option = {
    markdown_oxide = {
      keyword_pattern = [[\(\k\| \|\/\|#\)\+]]
    }
  }
},
```


I also recommend enabling codelens in neovim. Add this snippet to your on\_attach function for nvim-lspconfig


```lua
-- refresh codelens on TextChanged and InsertLeave as well
vim.api.nvim_create_autocmd({ 'TextChanged', 'InsertLeave', 'CursorHold', 'LspAttach' }, {
    buffer = bufnr,
    callback = vim.lsp.codelens.refresh,
})

-- trigger codelens refresh
vim.api.nvim_exec_autocmds('User', { pattern = 'LspAttached' })
```


1. Test it out! Go to definitions, get references, and more!

NOTE: To get references on files, you must place your cursor/pointer on the first character of the first line of the file, and then get references. (In VSCode, you can also use the references code lens)

## Note on Linking Syntax

The linking syntax is that of obsidian's and can be found here https://help.obsidian.md/Linking+notes+and+files/Internal+links

Generally, this is [[relativeFilePath(#heading)?(|display text|)?]] e.g. [[articles/markdown oxide#Features|Markdown Oxide Features]] to link to a heading in `Markdown Oxide.md` file in the `articles` folder or [[Obsidian]] for the `Obsidian.md` file in the root folder.  

## Features

- Go to definition (or definitions) from ...
    - [X] File references [[file]]
    - [X] Heading references [[file#heading]]
    - [X] Block references. [[file#^index]] (I call indexed blocks the blocks that you directly link to. The link will look like [[file#^index]]. When linking from the obsidian editor, an *index* ^index is appended to the paragraph/block you are referencing)
    - [X] Tags #tag and #tag/subtag/..
    - [X] Footnotes: "paraphrased text[^footnoteindex]"
    - [ ] Metadata tag
- Get references
    - [X] For File when cursor is on the first character of the first line of the file. This will produce references not only to the file but also to headings and blocks in the file
    - [X] For block when the cursor is on the blocks index "...text *^index*"
    - [X] For tag when the cursor is on the tags declaration. Unlike go to definition for tags, this will produce all references to the tag and to the tag with subtags
    - [X] Footnotes when the cursor is on the declaration line of the footnote; *[^1]: description...*
- Completions (requires extra nvim cmp config; follow the directions above)
    - [X] File link completions
    - [X] Heading link Completions
    - [ ] Subheading compeltions in the form [[file#heading#subheading]] from https://help.obsidian.md/Linking+notes+and+files/Internal+links#Link+to+a+heading+in+a+note (Note: right now you can link to subheadings through [[file#subheading]])
    - [X] Block link completions (searches the text of the block) 
    - [X] Footnote link completions
    - [X] New Block link Completions through grep: to use this, type `[[ `, and after you press space, completions for every block in the vault will appear; continue typing to fuzzy match the block that you want; finally, select the block; a link will be inserter to the text document and an index (ex ^1j239) will be appended to the block in its respective file
    - [ ] Callout/admonition completions
    - [ ] Metadata completions
    - [ ] Dataview completions
    - [ ] Metadata tag completions
    - [ ] \`\`\`query\`\`\` code block completions
- Hover Preview
    - [X] File
    - [X] Headings
    - [X] Indexed Blocks
    - [X] Footnotes
- [ ] Code Actions
    - [x] Unresolved file link -> Create the file
    - [x] Unresolved heading link -> append heading to file and create file
    - [ ] Link suggestions (by text match or other)
    - [ ] Refactoring: Move headers or selections to a new file
    - [ ] Link an unlinked reference
    - [ ] Link all unlinked references to a referenceable
- [X] Diagnostics
    - [X] Unresolved reference
    - [ ] Unlinked reference
- [X] Symbols
    - [X] File symbols: Headings and subheadings
    - [X] Workspace headings: everythign linkable: files, headings, tags, ... Like a good search feature
    - [ ] Lists and indented lists
- [ ] Rename
    - [X] File (cursor must be in the first character of the first line)
    - [X] Headings
    - [X] Tags
    - [ ] Indexed Blocks
- [ ] Dataview support
- [ ] Take some influence from LogSeq!!!!! https://docs.logseq.com/#/page/start%20here
    - [ ] Support Logseq syntax and completions/parsing for block references
    - [ ] Support Logseq embeds
    - [ ] Support Completions for logseq tasks
    - [ ] Support https://docs.logseq.com/#/page/markdown
    - [ ] Influence from logseq shortcut completions; such as to dates like /tomorrow
- Config
    * [ ] Daily notes format
- [ ] Proper integration tests
- A simple CLI
    - [ ] Working with daily notes (key to efficient PKM systems!)
    - [ ] Logsec tasks
    - [ ] ... (leave some ideas in the issues!)

# Alternatives

**I love open source and all open source authors!! I also believe healthy competition is good! Moxide is competing with some alternatives, and I want to make it the best at its job!!**

Here are the alternatives (open source authors are welcom to make PRs adding their projects here!)

- https://github.com/gw31415/obsidian-lsp ; I have been in discussions with the author; he/she is a med student and doesn't have time to maintain . I of course love his idea, but the current LS doesn't provide many obsidian specific features yet. 
- https://github.com/WhiskeyJack96/logseqlsp ; This is a cool project and a great inspiration for logseq support (which is upcoming). status: it doesn't seem that it is maintained; no commites for 3 months
- The og https://github.com/artempyanykh/marksman ; I used this for a while, but it is not obsidian specific and didn't act well w my vault


# ---The--bottom--line--------------------------------------------------------

Listen. I really like vim motions. I also really like low latency terminal editing. I very much so also like my neovim plugins and config. And wow I also like using obsidian (and other md apps). Can't I just have it all??? Can't I brute text edit in neovim and preview and fine edit in the JS madness? Well, I thought I could; that is why I am making this.
