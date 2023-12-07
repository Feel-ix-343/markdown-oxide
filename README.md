# obsidian-markdown-ls
Markdown language server for the obsidian note-taking format

Implementing obsidian PKM features (and possibly more) in the form of a language server allows us to use these features in our favorite text editors (neovim!) and reuse other lsp related plugins (like Telescope, outline, ...)

## Features

- [X] Go to definition (or definitions) from ...
    - [X] File references
    - [X] Heading references
    - [X] Indexed block references. (I call indexed blocks the blocks that you directly link to. The link will look like [[file#^index]]. When linking from the obsidian editor, an *index* ^index is appended to the paragraph/block you are referencing)
    - [X] Tags: This will get the locations where the tag is placed; it will give all the locations where the #tag/subtag is written. This is different than the functionality of the reference, which will get all tag and subtag usages: references on #tag will give #tag, #tag/subtag, #tag/sub/subtag ... 
    - [ ] Footnotes
    - [ ] Metadata tag
- [X] Get references
    - [X] To file
    - [X] to heading
    - [X] to indexed block
    - [X] to tag (explained above)
    - [ ] Footnotes
    - [ ] Metadata tag
- [ ] Completions
    - [ ] File, heading, tag completions, footnotes (named and numbered)
    - [ ] Indexed block completions. Somehow using Ripgrep to find the paragraphs/blocks in the vault, then appending an index in the file, then inserting a link
    - [ ] Callout completions
    - [ ] Metadata completions
    - [ ] Dataview?
    - [ ] Metadata tag
    - [ ] \`\`\`query\`\`\` code blocks
- [ ] Code Actions
    - [ ] Missing file for link -> Create the file
    - [ ] Link suggestions (by text match or other)
    - [ ] Refactoring: Move headers or selections to a new file
- [ ] Diagnostics
    - [ ] Missing link
- [ ] Outline
    - [ ] Headings and subheadings
    - [ ] Headings and subheadings of links
- [ ] Rename
    - [ ] File
    - [ ] Headings
    - [ ] Tags
- [ ] Dataview support?- [ ] 
