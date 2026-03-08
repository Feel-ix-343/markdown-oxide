# Markdown-Oxide

Markdown-Oxide is a Personal Knowledge Management System (PKMS) that composes with your favorite text-editor through the Language Server Protocol (LSP).

While other PKMS implementations include their own text-editors, markdown-oxide is *unbundled*: it leaves text-editing to a dedicated text-editor and focuses solely on robust, performant knowledge management.

It also is inspired by and highly compatible with Obsidian.

> [!note] Editor Support
> The best-supported text editor is Neovim, but also popular with users are VSCode, Helix, and Zed.
> 
> Markdown Oxide will work with any text editor implementing the Language Server Protocol, but support for features will depend on the extent to which the editor implements the LSP.

Markdown-oxide is for you if...
- You have a favorite text editor -- possibly one that you have spent days configuring and cannot live without -- and it supports the LSP.
- Your set of Personal Knowledge Management needs can be met by Markdown-oxide.

In this article, I give an overview of the features Markdown-oxide so that you can decide if it is for you. If you do decide you want to use it, there is a linked guide to help you set up and get comfortable with Markdown-oxide. 

# PKM Features

I will not cover all of Markdown-Oxide's features but instead, list how Markdown-oxide fulfills common PKM needs. If you are looking for all of the features we have, visit the [[Features Index]].

## Linking

We support links as a means of organization and provide several features to later use your linked notes

### Creating Links

You can create links between sections of documents through editor completions -- or Intellisense as VSCode calls it.

![[Features Index#^linking]]

![[Features Index#^unresolvedCompletions]]

### Using linked notes

- Use your editor's go-to-definition command to follow a Wiki or Markdown-link link
- Backlinks
    ![[Features Index#^backlinks]]
    * Also included are several enhancements to your editor's UI to view information on backlinks more easily. For example, we provide a code lens with the count of references to headings and files; it can be seen in some of the previews.
        + Note this does not work on Zed and Helix yet

### Editing linked notes

A challenge of linked notes is that they become difficult to edit. For example, changing a heading name will break links to the heading.

For this reason, we implement your editor's *rename* command so that you can rename files and headings as well as all related links.

![[Features Index#^renameLinked]]

## Daily Notes

### Navigating daily notes

By using Markdown-Oxide's LSP commands, you can navigate your daily notes very simply. 

Some examples of the commands in Neovim are

- `:Today`
- `:Tomorrow`
- `:Yesterday`
- `:Daily two days ago`
- `:Daily next monday`

### Linking to Daily Notes

![[Features Index#^implDailyNoteComp]]

> [!info]
> This allows you to give yourself reminders in the future. 
>
> Add a `[[next monday]]` link to a block and when you open your daily note on monday, you will see your block as a backlink

## Tags

### Adding Tags to files

Use tag completions to add previously defined tags to files

![[Features Index#^tagCompletions]]

### Using Tags

When you want to query your tagged files, you have the following options...

![[Features Index#^tagReferences]]

![[Features Index#^workspaceTag]]

- Find all references to a tag by typing the tag name into workspace symbols

## Extras

### Callout Completions

![[Features Index#^calloutCompletions]]

### Footnotes

#### Completions

![[Features Index#^footnoteCompletions]]

#### References

![[Features Index#^footnoteReferences]]

# Getting Started

If the features support your PKM needs and you have a desire to PKM in your favorite text editor, the setup guide is [here](<Setup Instructions.md>). I hope you enjoy it!

If Markdown-Oxide is not quite what you are looking for at this time, good luck on your PKM journey and consider checking back in the future!

> [!note] Github
> If you want to view the code, open pull-requests, participate in discussions, report bugs, and/or request features, visit the github repo: https://github.com/Feel-ix-343/markdown-oxide

[^1]: ![[rug/Documentation Notes#^docEmbeds]]
