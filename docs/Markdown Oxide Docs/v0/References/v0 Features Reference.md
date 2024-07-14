
When clicking on each drop down, you will be presented with a demo in of the feature in Neovim following [my configuration](https://github.com/Feel-ix-343/Neovim-Config) ^demoExpl

# Completions

## Implemented Completions Features

- ^implCompletion

    <details open>
      <summary>Wikilink Completions</summary>

    ![wikilinkcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/29c4830f-30e5-4094-9f5b-7b39009437da)
      
    </details>

    <details>
        <summary>Markdown Link Completions</summary>

    ![markdownlinkcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/16c8565a-6a28-4df1-a312-e4b158fb9f03)

        
    </details>

    <details>
        <summary>Block Completions: Fuzzy search through the whole folder of files and link anywhere, following obsidian block linking syntax</summary>

    to use this, type `[[`, and after you press space, completions for every block in the vault will appear; continue typing to fuzzy match the block that you want; finally, select the block; a link will be inserted to the text document and an index (ex ^1j239) will be appended to the block in its respective file. In Neovim, this text will not be written yet into the file (it will be edited in an unsaved buffer) so type `:wa`, and it should be resolved (as long as you have `dynamicRegistration = true` as described [here](https://github.com/Feel-ix-343/markdown-oxide?tab=readme-ov-file#neovim)!

    ![blockcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/a48c28a7-55b0-438c-becc-1dfde350fa94)
        
    </details>  

    <details>
        <summary>Tag Completions</summary>

    ![tagcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/bf20d7ac-171a-4d95-b510-ba323073c0b8)

        
    </details>

    <details>
        <summary>Footnote Completions</summary>

    ![footnotecompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/92a6739d-8a7a-457e-84bd-fde6548aa25a)
        
    </details>

    <details>
        <summary>Unresolved File and Heading Completions</summary>
        
    For those who like to reference things before they are written, `markdown-oxide` has terrific support for unresolved references! It provides completions for unresolved references, provides lsp_references for them, and provides code actions to create files + append headings.  


   ![unresolvedcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/96ed1a8e-eea3-4d3f-9557-e51b076fb3fb)

        
    </details>

    <details>
        <summary>Callout Completions</summary>

    ![calloutcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/11cd44f1-cf2e-4f27-92b4-1ed4914356ca)


        
    </details>

    <details>
        <summary>Nested Callout Completions</summary>

    ![nestedcalloutcompletions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/2ae86432-86fd-4327-b6e1-a94a5074db06)

        
    </details>

    <details>
        <summary>Alias Completions</summary>

    ![alias_completions](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/d83b2a6a-7b04-4cd4-92a2-ce78eccb4c3a)

        
    </details>


## Unimplemented Completions Features

- [ ] Subheading completions in the form [[file#heading#subheading]] from https://help.obsidian.md/Linking+notes+and+files/Internal+links#Link+to+a+heading+in+a+note (Note: right now you can link to subheadings through [[file#subheading]])
- [ ] Headings in the current file
- [ ] Metadata completions
- [ ] Dataview completions
- [ ] Metadata tag completions
- [ ] \`\`\`query\`\`\` code block completions
- [ ] Semantic Search unindexed block completions
- [ ] Contextual linking completions using vector database


# References

- ^implReference


    <details>
        <summary>File References: Gets references to the file and all headings and blocks in the file</summary>

    ![filereferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/9fbd6051-ef57-42eb-b61b-1cc3ddfb2293)
        
    </details>

    <details>
        <summary>Heading References</summary>

        
    ![headingreferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/50598628-ed27-4a9b-adba-861ca8f933ea)
        
    </details>

    <details>
        <summary>Tag References: Gets all references to the tag and subtags</summary>

    ![tagreferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/d73ac764-2c86-45c9-9403-17b50e6962e4)
        
    </details>

    <details>
        <summary>Indexed Block References</summary>

    ![indexedblockreferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/5d92257e-56b8-4209-b990-d25bbaa75a69)

        
    </details>

    <details>
        <summary>Footnote References</summary>

    ![footnotereferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/25940052-ca6c-4b7c-b334-f0001260c490)

    </details>

    <details>
        <summary>Unresolved file and heading references</summary>

    ![unresolvedreferences](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/5e5c89c1-fda0-4e80-98b5-3ccce4bd3dbf)


    </details>

> [!NOTE]
> If in neovim, I strongly recommend using [Lspsaga](https://github.com/nvimdev/lspsaga.nvim) for references for two reasons. First because this LS sorts references by the date their files were modified and unlike `vim.lsp.buf.references()` and `Telescope lsp_references`, `Lspsaga finder` maintains this sorting order. Second it also allows you to edit the references in place, similar to Logseq


# Hover

- ^implHover

    `markdown-oxide` provides a preview of the text for an item (if there is any) as well as a snapshot of the backlinks to the item (if applicable). You can hover over both references and referenceables -- hover over headings and links to headings; as well as files and links to files.

    In the hover, several backlines to the referenceable are listed, ordered by date modified.  

    > [!NOTE]
    > I write most of the content for a note not in the note itself, but in backlinks to the note; I also write in notes at times. Assuming content is both in backlinks and in written text, hover packages text and backlinks together to give a true preview of a referenceable. 

    <details>
        <summary>Gif of Hover for both references and referenceables</summary>

    ![hover](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/ed6d8d48-e700-42f2-8ab6-d0b8d2d038f9)

    </details>

# Code Actions

## Implemented Code Actions

- ^implCodeAction

    <details>
        <summary>Create file for unresolved file link</summary>

    ![codeactionsfile](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/707955e4-1e54-4f61-ac54-979d9f95b13c)


    </details> 

    <details>
        <summary>Append heading to file and create the file if necessary</summary>

        
    ![codeactionsheading](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/02af43aa-5185-406c-adb3-4c2792902761)



    </details>

## Future Code Actions Features

- [ ] Link suggestions (by text match or other)
- [ ] Refactoring: Move headers or selections to a new file
- [ ] Link an unlinked reference
- [ ] Link all unlinked references to a referenceable

# Diagnostics

## Implemented Diagnostics


- ^implDiagnostics

    Unresolved reference (no preview yet :( )

## Unimplemented Diagnostics

- [ ] Unlinked reference

# Symbols

## Implemented Symbols

- ^implSymbols

    File symbols: Headings and subheadings

    Workspace symbols: everything linkable: files, headings, tags, ... Like a good search feature

## Unimplemented Symbols

- [ ] Lists and indented lists


# Rename

- ^implRename

    <details>
        <summary>Rename File</summary>

    ![renamefile](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/3ac404fb-cfcd-4943-81ba-8ab3645831b7)


    </details>

    <details>
        <summary>Rename Heading</summary>

    ![renameheading](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/4227cd57-ca07-4d11-b6e8-afcaba554271)

    </details>

    <details>
        <summary>Rename Tag</summary>

    ![renametag](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/48b8a825-2342-477c-8440-198ab9273a83)


    </details>

# Daily Notes

- ^implDailyNoteComp

    Daily Note completions relative to the current date

    - <details>
        <summary>...for wikilinks</summary>

        ![dailynoteswiki](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/d2505535-ef5e-491a-bd88-ef12be2402ef)


    </details>

    - <details>
        <summary>...for markdown links</summary>

        ![dailynotesmd](https://github.com/Feel-ix-343/markdown-oxide/assets/88951499/23cf2f7c-1956-40b6-bfa9-0349c640516c)

    </details>


# LSP Workspace Commands

## Opening Daily Notes

![[v0/Guides/v0 Getting Started Guide#^8g4c9]]
