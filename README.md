# Markdown Oxide

Implementing Obsidian PKM features (and possibly more) in the form of a language server allows us to use these features in our favorite text editors and reuse other LSP-related plugins (like Telescope, outline, refactoring tools, ...). This language server primarily attempts to be a replacement and extension to the Obsidian markdown editor for Obsidian vault PKM-ing in your favorite text editing system -- creating the best PKM system for any text editor. 

## Installation

> [!IMPORTANT]
> I'm working on getting this into package distributions. Installation/configuration should be easier soon.


### Arch Linux

If you are using Arch Linux, you can install the latest Git version through the AUR. The package name is `markdown-oxide-git`. You can install it using your preferred AUR helper; for example:

```sh
paru -S markdown-oxide-git
```

### Cargo (Linux, MacOS, Windows)

If you have cargo installed, you can easily install the binary for the LS by running the following command:

```sh
cargo install --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
```

### Manual (MacOS, Windows, Linux)

Clone the repository and then run `cargo build --release`.

You will subsequently need the path to the release binary when you configure your editor. It can be found relative to the root of the project at `target/release/markdown-oxide`

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


*Test it out! Go to definitions, get references, and more!*

> [!NOTE]
> To get references on files, you must place your cursor/pointer on the first character of the first line of the file, and then get references. (In VSCode, you can also use the references code lens)

## Note on Linking Syntax

The linking syntax is that of Obsidian's and can be found here https://help.obsidian.md/Linking+notes+and+files/Internal+links

Generally, this is `[[relativeFilePath(#heading)?(|display text)?]]` e.g. [[articles/markdown oxide#Features|Markdown Oxide Features]] to link to a heading in `Markdown Oxide.md` file in the `articles` folder or [[Obsidian]] for the `Obsidian.md` file in the root folder.  

## Features

- Go to definition (or definitions) from ...
    - [X] File references [[file]]
    - [X] Heading references [[file#heading]]
    - [X] Block references. [[file#^index]] (I call indexed blocks the blocks that you directly link to. The link will look like [[file#^index]]. When linking from the obsidian editor, an *index* ^index is appended to the paragraph/block you are referencing)
    - [X] Tags #tag and #tag/subtag/\.\.
    - [X] Footnotes: "paraphrased text[^footnoteindex]"
    - [ ] Metadata tag
- Get references
    - [X] For File when the cursor is on the **first character of the first line** of the file. This will produce references not only to the file but also to headings and blocks in the file
    - [X] For block when the cursor is on the block's index "...text *^index*"
    - [X] For tag when the cursor is on the tags declaration. Unlike go-to-definition for tags, this will produce all references to the tag and to the tag with subtags
    - [X] Footnotes when the cursor is on the declaration line of the footnote; *[^1]: description...*
- Completions (requires extra nvim-cmp config; follow the directions above)
    - [X] File link completions
    - [X] Heading link Completions
    - [ ] Subheading completions in the form [[file#heading#subheading]] from https://help.obsidian.md/Linking+notes+and+files/Internal+links#Link+to+a+heading+in+a+note (Note: right now you can link to subheadings through [[file#subheading]])
    - [X] Block link completions (searches the text of the indexed block) 
    - [X] Footnote link completions
    - [X] New Block link Completions through grep: to use this, type `[[`, and after you press space, completions for every block in the vault will appear; continue typing to fuzzy match the block that you want; finally, select the block; a link will be inserted to the text document and an index (ex ^1j239) will be appended to the block in its respective file. In Neovim, this text will not be written yet into the file (it will be edited in an unsaved buffer) so type `:wa`, and it should be resolved (as long as you have `dynamicRegistration = true` as described [here](https://github.com/Feel-ix-343/markdown-oxide?tab=readme-ov-file#neovim)!
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
    - [X] Show backlinks, sorted by the date modified, in the hover (I will write most of the content for a note not in the note itself, but in backlinks to the note; I also will write in notes at times. This feature is to combine the content related to the note including both backlinks in actual organized text)
- [ ] Code Actions
    - [x] Unresolved file link -> Create the file
    - [x] Unresolved heading link -> append heading to file and create the file if necessary
    - [ ] Link suggestions (by text match or other)
    - [ ] Refactoring: Move headers or selections to a new file
    - [ ] Link an unlinked reference
    - [ ] Link all unlinked references to a referenceable
- [X] Diagnostics
    - [X] Unresolved reference
    - [ ] Unlinked reference
- [X] Symbols
    - [X] File symbols: Headings and subheadings
    - [X] Workspace headings: everything linkable: files, headings, tags, ... Like a good search feature
    - [ ] Lists and indented lists
- [ ] Rename
    - [X] File (cursor must be in the first character of the first line)
    - [X] Headings
    - [X] Tags
    - [ ] Indexed Blocks
- [ ] Dataview support
- Config
    * [ ] Daily notes format
- A simple CLI
    - [ ] Working with daily notes (key to efficient PKM systems!)
    - [ ] ... (leave some ideas in the issues!)
- [ ] Integrate with Obsidian.nvim

# Alternatives

**I love open-source and all open-source authors!! I also believe healthy competition is good! Markdown-Oxide is competing with some alternatives, and I want to make it the best at its job!!**

Here are the alternatives (open source authors are welcome to make PRs to add their projects here!)

- https://github.com/gw31415/obsidian-lsp: I have been in discussions with the author; The author doesn't have time to maintain the project. Also, I, of course, love the idea, but the current LS doesn't provide many obsidian-specific features yet.
- https://github.com/WhiskeyJack96/logseqlsp: This is a cool project and a great inspiration for Logseq support (which is upcoming). status: it doesn't seem that it is maintained and it (obviously) does not provide support for all of the obsidian syntax
- The og https://github.com/artempyanykh/marksman: I used this for a while, but it is not obsidian specific and didn't act well with my vault. Additionally, the block completions in markdown-oxide allow for a fuzzy/grep search of the entire vault to generate the completions; I don't think Markman has any features like this; (this is a feature that Logseq signified for PKM; the concept that *anything is linkable* is quite powerful) 


# ---The--bottom--line--------------------------------------------------

Listen. I really like Vim motions. I also really love low-latency terminal editing. I very much so also like my Neovim LSP plugins, keymappings, and config. But Wow! I also like using Obsidian and Logseq. **Can't I just have it all???** Can't I be whisked away by the flow of Neovim while also experiencing the beauty of Obsidian???? Can't I detail my tasks on the CLI while viewing them in Logseq????? Well, I thought I could; for us all, there is markdown-oxide (which is still very pre-beta hah)
