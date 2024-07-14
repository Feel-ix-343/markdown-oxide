

![[What is Markdown Oxide?]]

Markdown Oxide version 0 was created to enable users to do pre-determined, Obsidian-inspired, PKM workflows in the comfort of their favorite text editor. v0 was not made for those new to PKMing. For this reason, this guide will not show many potential workflows or explain much about PKMing and rather should be used to determine _if_ v0 is for you (and your text editor(s)) based on how you currently PKM.[^1]

# Features

Look through v0's current features and determine if it can facilitate your current workflow. If it can, continue to the [setup](<v0 Getting Started Guide#Setup>)


> [!note]
> - MXIDE v0 is *only* a language server, and all of the features are directly alligned to implementations of the [langauge server protocol](https://microsoft.github.io/language-server-protocol/)
> - Editor support info is included. Neovim (with the specified plugins) and VSCode support all features; other editors may not. 
> - ![[v0 Features Reference#^demoExpl]]

## Completions 

> [!info]- Editor Support info
> - [Neovim docs](https://neovim.io/doc/user/lsp.html#_lua-module:-vim.lsp.completion)
> > - [Pretty Completion Plugin](https://github.com/hrsh7th/nvim-cmp)
> - [VSCode Intellisense](https://code.visualstudio.com/docs/editor/intellisense)
> - Zed supports completions. However, because it does not support LSP Workspace Commands, block completions (described below) will not work. I have opened an [issue](https://github.com/zed-industries/zed/issues/13756) for this
> - Helix supports completions, though you will need to manually re-request completions as you type. I have opened a [issue](https://github.com/helix-editor/helix/issues/9797) about this. Helix also does not support block completions. 

![[v0 Features Reference#^implCompletion]]

![[v0 Features Reference#^implDailyNoteComp]]

## References


> [!info]- Editor Support Info
> - Neovim supports this, but the support is greatly aided by plugins
> > - [Telescope](https://github.com/nvim-telescope/telescope.nvim) is the one I use in the demo
> > - [Lspsaga](https://github.com/nvimdev/lspsaga.nvim)
> 
> All others support

![[v0 Features Reference#^implReference]]

## Hover

> [!info]- Editor Support Info
> All editors support this well
> 
> Note that in VSCode and Zed, it may be helpful to [disable hover](<v0 Configuration Reference>) so that hovering with the mouse does not trigger it. In neovim and helix, hover is done with keycommands, so this is not an issue. 

![[v0 Features Reference#^implHover]]

## Code Actions


> [!info]- Editor Support Info
> All editors support this well

![[v0 Features Reference#^implCodeAction]]

## Diagnostics

> [!info]- Editor Support Info
> All editors support this well

![[v0 Features Reference#^implDiagnostics]]

## Symbols

> [!info]- Editor Support Info
> All editors support this well


![[v0 Features Reference#^implSymbols]]

## Rename


> [!info]- Editor Support Info
> All editors other than Zed support this well

![[v0 Features Reference#^implRename]]

## Workspace Commands

> [!info]- Editor Support Info
> - Neovim has full support for these commands, including the [predefined daily note commands]() and the natrual langauge commands
> - I have not yet figured out how to use these in VSCode, but I am sure that at least the [predefined daily note commands]() are possible

- Opening Daily Notes   ^8g4c9
    * Open or create daily notes through a natural language relative name. 
        + The full specification for the relative name is [here](<Daily Notes#Opening Daily Notes>)
        + Some examples of this command in neovim following the [Neovim Setup](<v0 Getting Started Guide#Installation and Possible Editor Config>) below: ![[Daily Notes#^nvimrelativenamescmds]]
    * Open or create daily notes through predefined relative names. 
        + The names are as follow: ![[Daily Notes#^predefinedNames]]
        + Each of these names have their own workspace commands


# Setup 

Follow these steps to install v0 in your favorite text editor.

## Installation and Possible Editor Config

### ![[Setup#Neovim]]

### ![[Setup#VSCode]]

### ![[Setup#Helix]]

### ![[Setup#Zed]]

## Configuration

![[v0 Configuration Reference#^configurationinfo]]

You can probably leave all of the settings as their defaults for now, but here are a few you may want to set.

- [Daily Note Format](<v0 Configuration Reference#Daily Note Format Config Option>). If you have obsidian, this one is automatically imported if not set. 
- Do you *not* use the first heading of your markdown files as the file title? You probably want to disable [[v0 Configuration Reference#Title Headings|title headings]].
- Do you have a specific folder where your daily notes are? Try [[v0 Configuration Reference#Daily Notes Folder]]. *This may be imported from Obsidian*
- Do you have a specific folder where new files should be created? Try [[v0 Configuration Reference#New Files Folder]]. *This may be imported from Obsidian*
- Do you want `.md` parsed and appended in your links? [[v0 Configuration Reference#MD Extension]]

And some more helpful information
- [[v0 Configuration Reference#Default Config File]]
- For info on all v0 settings: [[v0 Configuration Reference]].

I hope you enjoy!

[^1]: ![[Documentation Notes#^docEmbeds]]
