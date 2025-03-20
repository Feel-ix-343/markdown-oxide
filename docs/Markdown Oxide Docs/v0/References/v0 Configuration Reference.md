`Markdown-Oxide` supports several configuration options. All can be specified in a `~/.config/moxide/settings.toml` or `.moxide.toml` file. Moxide also tries to import some settings from Obsidian directly. ^configurationinfo

# Default Config File

This contains all possible settings with brief descriptions. A bit of elaboration on the settings is included after.

```toml
# Leave blank to try to import from Obsidian Daily Notes
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

# The folder for new files to be created in; this is relevant for the code action that creates
# from an unresolved link. If not specified, it will import from your obsidian config option titled
# "Default Location for new notes" -- which is "" by default too.
new_file_folder_path = ""


# The folder for new daily notes: this is applied for the create file for unresolved link code action
# as well as the Today, Tomorrow, Yesterday, and Daily... lsp commands
#
# This is also imported from obsidian if not specified: specifically the option titled "New file location"
daily_notes_folder = ""


# Whether markdown links should include an extension or not
# for example [File](file.md) or [File](file)
include_md_extension_md_link = false

# Whether wikilinks should include an extension or not (needed for Markor compatibility)
# for example [[File]] or [[File.md]]
include_md_extension_wikilink = false

# Enable hover; this is relevant for VSCode and Zed where hover could be triggered on mouse hover
# and could be annoying
hover = true

# Handle case in fuzzy matches: Ignore | Smart | Respect
case_matching = "Smart"

# Enable inlay hints
inlay_hints = true
# Enable transclusion, in the form of inlay hints, for embedded block links: ![[link]]
# Inlay hints must be enabled
block_transclusion = true
# Full or Partial, for Partial, block_transclusion_length = { partial = 10 }
# block_transclusion must be enabled for this to take effect
block_transclusion_length = "Full"
```

# Daily Note Format Config Option

```toml
dailynote = "{format}"
```

The format uses [Chrono's date formatting syntax](https://docs.rs/chrono/latest/chrono/format/strftime/index.html):

- Default: `%Y-%m-%d` (equivalent to YYYY-MM-DD in Moment.js)
- This format is used unless imported from Obsidian

When importing from Obsidian, Markdown Oxide automatically converts Moment.js format strings to Chrono format ![[#^1862g]]

## Relevance

This configuration is used in several features:

- Generating [date completions in links](</Users/markayers/source/x3c3/markdown-oxide/docs/Markdown Oxide Docs/Daily Notes.md#Using Relative Names in Links>) with predefined relative date expressions like "today" and "tomorrow"
- Navigating to and creating [daily notes](</Users/markayers/source/x3c3/markdown-oxide/docs/Markdown Oxide Docs/Daily Notes.md#Navigating to Daily Notes>) with the `:Daily` command and similar features
- [Creating files from unresolved references](</Users/markayers/source/x3c3/markdown-oxide/docs/Markdown Oxide Docs/v0/References/v0 Features Reference.md#Code Actions>) when the links match the daily note format

## Date Formatting

For a comprehensive guide to date formatting options, see:

![[Date formatting]]

# Settings From Obsidian

- ... ^someobsidiansettings
  - Daily Note:
    - `dailynote`: checks if you have the dailynote Obsidian plugin and translates this formatting to Markdown Oxide's date formatting ^1862g
    - Info on this date formatting can be found [here](<Date Formatting>)
  - `new_file_folder_path`: uses the specific folder for new files you set in Obsidian if you have it enabled. This is relevant to the [Create Unresolved File Code Action](<v0 Features Reference#^implCodeAction>)
  - `daily_notes_folder_path`: uses the specific folder for new daily notes you set in the Obsidian Daily Notes plugin, if you have this option enabled. This is relevant to the path for [opening daily notes](<v0 Features Reference#Opening Daily Notes>) and for [the code action that creates unresolved links](<v0 Features Reference#^implCodeAction>) if they have the `dailynote` format.
