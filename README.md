# obsidian-markdown-ls
Obsidian Language Server: Obsidian-flavored-markdown language server 
Implementing obsidian PKM features (and possibly more) in the form of a language server allows us to use these features in our favorite text editors (neovim or vscode!) and reuse other lsp related plugins (like Telescope, outline, and builtin lsp support)

## Installation for Neovim (there is no VS code plugin yet)

Make sure rust is installed properly and that you are using nvim cmp (I am not sure if it works in other completion engines)

1. Clone the repo
2. `Cargo build --release`
3. Add and adjust the following to your Neovim config  

```
local configs = require("lspconfig.configs")
configs["obsidian_ls"] = {
default_config = {
  root_dir = function() return vim.fn.getcwd() end,
  filetypes = {"markdown"},
  cmd = {"{path}"} -- replace {path} with the path to the --release build. 
  -- {path} will be {where ever you cloned from}/obsidian-ls/target/release/obsidian-ls
},
on_attach = on_attach, -- do this only if you have an on_attach function already
capabilities = capabilities, -- add the nvim cmp capabilities if using it
}
require("lspconfig").obsidian_ls.setup({})
```

then adjust your nvim-cmp source settings for the following. Note that this will likely change in the future.

```
{
    name = 'nvim_lsp',
         option = {
             obsidian_ls = {
                 keyword_pattern = [[\(\k\| \|\/\|#\)\+]]
             }
         }
},
```


1. Test it out! Go to definitions, get references, and more!

## Features

- [ ] Go to definition (or definitions) from ...
    - [X] File references
    - [X] Heading references
    - [X] Indexed block references. (I call indexed blocks the blocks that you directly link to. The link will look like [[file#^index]]. When linking from the obsidian editor, an *index* ^index is appended to the paragraph/block you are referencing)
    - [X] Tags: This will get the locations where the tag is placed; it will give all the locations where the #tag/subtag is written. This is different than the functionality of the reference, which will get all tag and subtag usages: references on #tag will give #tag, #tag/subtag, #tag/sub/subtag ... 
    - [X] Footnotes
    - [ ] Metadata tag
- [ ] Get references
    - [X] To file
    - [X] to heading
    - [X] to indexed block
    - [X] to tag (explained above)
    - [X] Footnotes
    - [ ] Metadata tag
- [ ] Completions
    - [X] File completions (requires extra nvim cmp configuration)
    - [X] Heading Completions (requires extra nvim cmp config)
    - [X] Indexed block completions. Somehow using Ripgrep to find the paragraphs/blocks in the vault, then appending an index in the file, then inserting a link (workaround supported; using "_" instead of any non_word characters)
    - [X] Footnotes
    - [ ] Create file completions
    - [ ] Make file completions faster in NvimCmp (this is mostly a neovim issue as moxide generates very many completions for large vaults)
    - [ ] Callout completions
    - [ ] Metadata completions
    - [ ] Dataview?
    - [ ] Metadata tag
    - [ ] \`\`\`query\`\`\` code blocks
- [X] Preview
    - [X] File
    - [X] Headings
    - [X] Indexed Blocks
    - [X] Footnotes
- [ ] Code Actions
    - [x] Missing file for link -> Create the file
    - [ ] Link suggestions (by text match or other)
    - [ ] Refactoring: Move headers or selections to a new file
- [X] Diagnostics
    - [X] Missing reference
- [X] Symbols
    - [X] File symbols: Headings and subheadings
    - [X] Workspace headings: everythign linkable: files, headings, tags, ... Like a good search feature
    - [ ] Lists and indented lists
- [ ] Rename
    - [ ] File
    - [ ] Headings
    - [ ] Tags.
    - [ ] Indexed Blocks
- [ ] Dataview support
- [ ] Take some influence from LogSeq!!!!! https://docs.logseq.com/#/page/start%20here
    - [ ] Support Logseq syntax and completions/parsing for block references



### Dev Todo's

- [X] Extract the RwLock read pattern
- [ ] Fix: updating diagnostics after code action


# Workflows

- Linking to daily notes in the future to set todos; checking references to those notes using the language server



# The bottom line

Listen; I really like vim motions. I also really like low latency terminal editing. I very much so also like my neovim plugins. And wow I also like using obsidian (and other md apps). Can't I just have it all??? Can't I brute edit in neovim and preview and fine edit in the JS madness? Well, I thought I could; that is why I am making this. (And hopefully why you will help me!)
