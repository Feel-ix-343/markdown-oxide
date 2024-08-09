

# What is Markdown-Oxide?

![[What Is Markdown-Oxide#^whatIsMarkdownOxide]]

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

I will not cover all of Markdown-Oxide's features but instead, list how Markdown-oxide fulfills common PKM needs. If you are looking for all of the features we have, visit the [[v0 Features Reference]].

## Linking

We support links as a means of organization and provide several features to later use your linked notes

### Creating Links

You can create links between sections of documents through editor completions -- or Intellisense as VSCode calls it.

![[v0 Features Reference#^linking]]

![[v0 Features Reference#^unresolvedCompletions]]

### Using linked notes

- Use your editor's go-to-definition command to follow a Wiki or Markdown-link link
- Backlinks
    ![[v0 Features Reference#^backlinks]]
    * Also included are several enhancements to your editor's UI to view information on backlinks more easily. For example, we provide a code lens with the count of references to headings and files; it can be seen in some of the previews.
        + Note this does not work on Zed and Helix yet

### Editing linked notes

A challenge of linked notes is that they become difficult to edit. For example, changing a heading name will break links to the heading.

For this reason, we implement your editor's *rename* command so that you can rename files and headings as well as all related links.

![[v0 Features Reference#^renameLinked]]

## Daily Notes

### Navigating daily notes

By using Markdown-Oxide's LSP commands, you can navigate your daily notes very simply. 

Some examples of the commands in Neovim are

- `:Today`
- `:Tomorrow`
- `:Yesterday`
- `:Daily two days ago`
- `:Daily two days ago`



### Linking to Daily Notes

![[v0 Features Reference#^implDailyNoteComp]]

> [!info]
> This allows you to give yourself reminders in the future. 
>
> Add a `[[next monday]]` link to a block and when you open your daily note on monday, you will see your block as a backlink

## Tags

### Adding Tags to files

Use tag completions to add previously defined tags to files

![[v0 Features Reference#^tagCompletions]]

### Using Tags

When you want to query your tagged files, you have the following options...

![[v0 Features Reference#^tagReferences]]

![[v0/References/v0 Features Reference#^workspaceTag]]

- Find all references to a tag by typing the tag name into workspace symbols

## Extras

### Callout Completions

![[v0 Features Reference#^calloutCompletions]]

### Footnotes

#### Completions

![[v0 Features Reference#^footnoteCompletions]]

#### References

![[v0/References/v0 Features Reference#^footnoteReferences]]


# Getting Started

If the features support your PKM needs and you have a desire to PKM in your favorite text editor, the setup guide is [here](<Getting started with Markdown Oxide Version 0>). I hope you enjoy it!

If Markdown-Oxide is not quite what you are looking for at this time, good luck on your PKM journey and consider checking back in the future!

-[Felix](<Felix Zeller>)



[^1]: ![[Documentation Notes#^docEmbeds]]
