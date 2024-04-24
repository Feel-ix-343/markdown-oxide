# Markdown Oxide

Markdown Oxide is attempting to be the best PKM system for software enthusiasts - people like me who (in addition to note-taking) are addicted to creating the best text editing experience. 

Markdown Oxide's PKM features are strongly inspired by Obsidian - in fact Markdown Oxide is fully compatible with your Obsidian vault. Markdown Oxide does not aim to fully replace Obsidian; it serves to provide a feature rich and advanced note taking experience. Obsidian remains a terrific front-end for your linked markdown notes. Also, in terms of features, Markdown Oxide and Obsidian are quite alligned.

Markdown Oxide's features are implemented in the form of a language server aiming to be fully compatible with your favorite text editor and its ecosystem. Read on to learn what Markdown Oxide provides and how to install and configure it. 

## Installation

(if you want to skip to the features, [click here](https://github.com/Feel-ix-343/markdown-oxide?tab=readme-ov-file#features))

### Neovim

1. Given neovim access to the binary.

    - <details>
         <summary>Cargo Install (from source)</summary>
    
        ```bash
        cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
        ```
    
    </details>
    
    - <details>
         <summary>AUR (from source)</summary>
    
        ```bash
        paru -S markdown-oxide-git
        ```

        ```bash
        yay -S markdown-oxide-git
        ```
    
    </details>

    - [Mason.nvim](https://github.com/williamboman/mason.nvim) (from hosted binary)
    - Nix Unstable: `pkgs.markdown-oxide`
  
2. Modify your Neovim Configuration
    - <details>
        <summary>Modify LSP Config (making sure to adjust capabilities as follows)</summary>

        ```lua        
        -- An example nvim-lspconfig capabilities setting
        local capabilities = require("cmp_nvim_lsp").default_capabilities(vim.lsp.protocol.make_client_capabilities())
        
        -- Ensure that dynamicRegistration is enabled! This allows the LS to take into account actions like the
        -- Create Unresolved File code action, resolving completions for unindexed code blocks, ...
        capabilities.workspace = {
            didChangeWatchedFiles = {
              dynamicRegistration = true,
            },
        }
        
        require("lspconfig").markdown_oxide.setup({
            capabilities = capabilities, -- again, ensure that capabilities.workspace.didChangeWatchedFiles.dynamicRegistration = true
            on_attach = on_attach -- configure your on attach config
        })
        ```

    </details> 

    - <details>
        <summary>Modify your nvim-cmp configuration</summary>

        Modify your nvim cmp source settings for nvim-lsp (note: you must have nvim-lsp installed)

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

    </details>

    - <details>
        <summary>(optional) Enable Code Lens (eg for UI reference count)</summary>

        Modify your lsp `on_attach` function.

        ```lua
        -- refresh codelens on TextChanged and InsertLeave as well
        vim.api.nvim_create_autocmd({ 'TextChanged', 'InsertLeave', 'CursorHold', 'LspAttach' }, {
            buffer = bufnr,
            callback = vim.lsp.codelens.refresh,
        })
        
        -- trigger codelens refresh
        vim.api.nvim_exec_autocmds('User', { pattern = 'LspAttached' })
        ```

    </details>


### VSCode

Install the [vscode extension](https://marketplace.visualstudio.com/items?itemName=FelixZeller.markdown-oxide) (called `Markdown Oxide`). As for how the extension uses uses the language server, there are two options
1. Recommended: the extension will download the server's binary and use that
2. The extension will use `markdown-oxide` from path. To install to your path, there are the following methods for VSCode:

    - <details>
         <summary>Cargo Install (from source)</summary>
    
        ```bash
        cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
        ```
    
    </details>
    
    - <details>
         <summary>AUR (from source)</summary>
    
        ```bash
        paru -S markdown-oxide-git
        ```

        ```bash
        yay -S markdown-oxide-git
        ```
    
    </details>
    
    - Nix Unstable: `pkgs.markdown-oxide`


### Zed

Markdown Oxide is availiable as an extenion titled `Markdown Oxide`. Similarly to VSCode, there are two methods to for this extension to access the language server
1. Recommended: the extension will download the server's binary and use that
2. The extension will use `markdown-oxide` from path. To install to your path, there are the following methods for Zed:

    - <details>
         <summary>Cargo Install (from source)</summary>
    
        ```bash
        cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
        ```
    
    </details>
    
    - <details>
         <summary>AUR (from source)</summary>
    
        ```bash
        paru -S markdown-oxide-git
        ```

        ```bash
        yay -S markdown-oxide-git
        ```
    
    </details>
    
    - Nix Unstable: `pkgs.markdown-oxide`

    
> [!Note]
> Zed does not implement some of the language server protocol that this LS uses. Namely, unindexed block completions do not work at all. There are also other issues with the language server unique to Zed (such as completions being unexpectedly hidden). Overtime, these issue will be resolved; for now, Zed provides an interesting exhibition for a potential note taking experience provided by markdown oxide

### Helix

For Helix, all you must do is install the language server's binary to your path. The following installation methods are availiable:
- <details>
     <summary>Cargo Install (from source)</summary>

    ```bash
    cargo install --locked --git https://github.com/Feel-ix-343/markdown-oxide.git markdown-oxide
    ```

</details>

- <details>
     <summary>AUR (from source)</summary>

    ```bash
    paru -S markdown-oxide-git
    ```

    ```bash
    yay -S markdown-oxide-git
    ```

</details>

- Nix Unstable: `pkgs.markdown-oxide`


> [!Note]
> There are some major issue with markdown oxide on helix as it does not fully implement the language server protocol. Most obtrusive is that helix does not implement `is_incomplete` for completions, and since completion filtering and sorting happens on the server (for performance), you must manually rerequest completions after typing. 


## Linking Syntax

The linking syntax is that of Obsidian's and can be found here https://help.obsidian.md/Linking+notes+and+files/Internal+links

Generally, this is `[[relativeFilePath(#heading)?(|display text)?]]` e.g. [[articles/markdown oxide#Features|Markdown Oxide Features]] to link to a heading in `Markdown Oxide.md` file in the `articles` folder or [[Obsidian]] for the `Obsidian.md` file in the root folder. Markdown oxide also support markdown links

## Features

> [!NOTE]
> To interact with a file as a referenceable (for getting references, renaming, hover-view, ...), put your cursor/pointer anywhere on the markdown fide where there is not another referenceable (heading, tag, ...). 

### Completions

- <details>
  <summary>Wikilink Completions</summary>

  ![wikilinkcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/29c4830f-30e5-4094-9f5b-7b39009437da)
  
</details>

- <details>
    <summary>Markdown Link Completions</summary>

    ![markdownlinkcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/16c8565a-6a28-4df1-a312-e4b158fb9f03)

    
</details>

- <details open>
    <summary>Unindexed Block Completions; Fuzzy search through the whole folder of files and link anywhere, following obsidian block linking syntax</summary>

    to use this, type `[[`, and after you press space, completions for every block in the vault will appear; continue typing to fuzzy match the block that you want; finally, select the block; a link will be inserted to the text document and an index (ex ^1j239) will be appended to the block in its respective file. In Neovim, this text will not be written yet into the file (it will be edited in an unsaved buffer) so type `:wa`, and it should be resolved (as long as you have `dynamicRegistration = true` as described [here](https://github.com/Feel-ix-343/markdown-oxide?tab=readme-ov-file#neovim)!

    ![blockcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/a48c28a7-55b0-438c-becc-1dfde350fa94)
    
</details>


- <details>
    <summary>Tag Completions</summary>

    ![tagcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/bf20d7ac-171a-4d95-b510-ba323073c0b8)

    
</details>

- <details>
    <summary>Footnote Completions</summary>

    ![footnotecompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/92a6739d-8a7a-457e-84bd-fde6548aa25a)
    
</details>

- <details>
    <summary>Unresolved File and Heading Completions</summary>
    
    For those who like to reference things before they are written, `markdown-oxide` has terrific support for unresolved references! It provides completions for unresolved references, provides lsp_references for them, and provides code actions to create files + append headings.  


   ![unresolvedcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/96ed1a8e-eea3-4d3f-9557-e51b076fb3fb)

    
</details>

- <details>
    <summary>Callout Completions</summary>

    ![calloutcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/11cd44f1-cf2e-4f27-92b4-1ed4914356ca)


    
</details>

- <details>
    <summary>Nested Callout Completions</summary>

    ![nestedcalloutcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/2ae86432-86fd-4327-b6e1-a94a5074db06)

    
</details>

- <details>
    <summary>Alias Completions</summary>

    ![alias_completions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/d83b2a6a-7b04-4cd4-92a2-ce78eccb4c3a)

    
</details>

- [ ] Subheading completions in the form [[file#heading#subheading]] from https://help.obsidian.md/Linking+notes+and+files/Internal+links#Link+to+a+heading+in+a+note (Note: right now you can link to subheadings through [[file#subheading]])
- [ ] Callout/admonition completions
- [ ] Metadata completions
- [ ] Dataview completions
- [ ] Metadata tag completions
- [ ] \`\`\`query\`\`\` code block completions
- [ ] Semantic Search unindexed block completions
- [ ] Contextual linking completions using vector database


### References

- <details>
    <summary>File References: Gets references to the file and all headings and blocks in the file</summary>

    ![filereferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/9fbd6051-ef57-42eb-b61b-1cc3ddfb2293)
    
</details>

- <details>
    <summary>Heading References</summary>

    
    ![headingreferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/50598628-ed27-4a9b-adba-861ca8f933ea)
    
</details>

- <details>
    <summary>Tag References: Gets all references to the tag and subtags</summary>

    ![tagreferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/d73ac764-2c86-45c9-9403-17b50e6962e4)
    
</details>

- <details>
    <summary>Indexed Block References</summary>

    ![indexedblockreferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/5d92257e-56b8-4209-b990-d25bbaa75a69)

    
</details>

- <details>
    <summary>Footnote References</summary>

    ![footnotereferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/25940052-ca6c-4b7c-b334-f0001260c490)

</details>

- <details>
    <summary>Unresolved file and heading references</summary>


    ![unresolvedreferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/5e5c89c1-fda0-4e80-98b5-3ccce4bd3dbf)


</details>

> [!NOTE]
> I strongly recommend using [Lspsaga](https://github.com/nvimdev/lspsaga.nvim) for references for two reasons. First because this LS sorts references by the date their files were modified and unlike `vim.lsp.buf.references()` and `Telescope lsp_references`, `Lspsaga finder` maintains this sorting order. Second it also allows you to edit the references in place, similar to Logseq


### Hover

`markdown-oxide` provides a preview of the text for an item (if there is any) as well as a snapshot of the backlinks to the item (if applicable). You can hover over both references and referenceables -- hover over headings and links to headings; as well as files and links to files.

In the hover, several backlines to the referenceable are listed, ordered by date modified.  

> [!NOTE]
> I write most of the content for a note not in the note itself, but in backlinks to the note; I also write in notes at times. Assuming content is both in backlinks and in written text, hover packages text and backlinks together to give a true preview of a referenceable. 

<details>
    <summary>Gif of Hover for both references and referenceables</summary>

![hover](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/ed6d8d48-e700-42f2-8ab6-d0b8d2d038f9)

</details>

### Code Actions

- <details>
    <summary>Create file for unresolved file link</summary>

    ![codeactionsfile](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/707955e4-1e54-4f61-ac54-979d9f95b13c)


</details>

- <details>
    <summary>Append heading to file and create the file if necessary</summary>

    
    ![codeactionsheading](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/02af43aa-5185-406c-adb3-4c2792902761)



</details>

- [ ] Link suggestions (by text match or other)
- [ ] Refactoring: Move headers or selections to a new file
- [ ] Link an unlinked reference
- [ ] Link all unlinked references to a referenceable

### Diagnostics

- [X] Unresolved reference
- [ ] Unlinked reference

### Symbols

- [X] File symbols: Headings and subheadings
- [X] Workspace headings: everything linkable: files, headings, tags, ... Like a good search feature
- [ ] Lists and indented lists


### Rename


- <details>
    <summary>Rename File</summary>

    ![renamefile](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/3ac404fb-cfcd-4943-81ba-8ab3645831b7)


</details>


- <details>
    <summary>Rename Heading</summary>

    ![renameheading](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/4227cd57-ca07-4d11-b6e8-afcaba554271)

</details>

- <details>
    <summary>Rename Tag</summary>

    ![renametag](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/48b8a825-2342-477c-8440-198ab9273a83)


</details>

### Daily Notes

Daily Note completions relative to the current date

- <details>
    <summary>...for wikilinks</summary>

    ![dailynoteswiki](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/d2505535-ef5e-491a-bd88-ef12be2402ef)


</details>

- <details>
    <summary>...for markdown links</summary>

    ![dailynotesmd](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/23cf2f7c-1956-40b6-bfa9-0349c640516c)

</details>


## Config

`Markdown-Oxide` supports several configuration options. All can be specified in a `~/.config/moxide/settings.toml` or `.moxide.toml` file and moxide tries to import some settings (daily notes formatting) from Obsidian directly. Here are the options with the defaults

```toml
# Leave blank to try to import from Obsidian Daily Notes
# Formatting from https://docs.rs/chrono/latest/chrono/format/strftime/index.html
dailynote = "%Y-%m-%d" # this is akin to YYYY-MM-DD from Obsidian

# Fuzzy match file headings in completions
heading_completions = true

# Set true if you title your notes by the first heading
# Right now, if true this will cause completing a file link in the markdown style
# to insert the name of the first heading in the display text area
# [](file) -> [first heading of file.md](file)
# If false, [](file) -> [](file) (for example)
title_headings = true

# Show diagnostics for unresolved links; note that even if this is turned off, 
# special semantic tokens will be sent for the unresolved links, allowing you
# to visually identify unresolved links
unresolved_diagnostics = true

semantic_tokens = true

# Resolve tags in code blocks
tags_in_codeblocks = true
# Resolve references in code blocks
references_in_codeblocks = true
```


## Alternatives

**I love open-source and all open-source authors!! I also believe healthy competition is good! Markdown-Oxide is competing with some alternatives, and I want to make it the best at its job!!**

Here are the alternatives (open source authors are welcome to make PRs to add their projects here!)

- https://github.com/gw31415/obsidian-lsp: I have been in discussions with the author; The author doesn't have time to maintain the project. Also, I, of course, love the idea, but the current LS doesn't provide many obsidian-specific features yet.
- https://github.com/WhiskeyJack96/logseqlsp: This is a cool project and a great inspiration for Logseq support (which is upcoming). status: it doesn't seem that it is maintained and it (obviously) does not provide support for all of the obsidian syntax
- The og https://github.com/artempyanykh/marksman: I used this for a while, but it is not obsidian specific and didn't act well with my vault. Additionally, the block completions in markdown-oxide allow for a fuzzy/grep search of the entire vault to generate the completions; I don't think Markman has any features like this; (this is a feature that Logseq signified for PKM; the concept that *anything is linkable* is quite powerful) 


## ---The--bottom--line--------------------------------------------------

Listen. I really like Vim motions. I also really love low-latency terminal editing. I very much so also like my Neovim LSP plugins, keymappings, and config. But Wow! I also like using Obsidian and Logseq. **Can't I just have it all???** Can't I be whisked away by the flow of Neovim while also experiencing the beauty of Obsidian???? Can't I detail my tasks on the CLI while viewing them in Logseq????? Well, I thought I could; for us all, there is markdown-oxide (which is still very pre-beta hah)
