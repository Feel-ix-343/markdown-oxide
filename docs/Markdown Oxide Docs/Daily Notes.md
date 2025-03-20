# Daily Notes in Markdown Oxide

Daily notes are a cornerstone of many knowledge management systems. Markdown Oxide makes creating, navigating, and linking to daily notes seamless across any supported editor.

## Configuration

Daily notes follow a configurable date format (default: `%Y-%m-%d`) and can be stored in a dedicated folder. You can customize:

- Date format pattern
- Daily notes folder location
- Default location for new daily notes

These settings can be configured either through a `.moxide` config file in your vault root or through your editor's settings.

## Using Relative Names in Links

Markdown Oxide provides smart completions for linking to daily notes using natural language date references.

### Predefined Relative Names

![[Daily Notes#^predefinedNames]]

These can be used in completions by typing:

- `[[{relativename}` for wiki-links
- `[{display?}]({relativename})` for markdown links

### Examples

When you type these patterns and select the completion:

| What you type         | What gets inserted       |
| --------------------- | ------------------------ |
| `[[today`             | `[[2024-07-14]]`         |
| `[[tomorrow`          | `[[2024-07-15]]`         |
| `[](tomorrow`         | `[tomorrow](2024-07-15)` |
| `[Tomorrow](tomorrow` | `[Tomorrow](2024-07-15)` |

Note: The display text in markdown links is preserved when you select a completion.

## Navigating to Daily Notes

Markdown Oxide provides powerful navigation capabilities for daily notes using natural language date expressions.

### Natural Language Date Support

Markdown Oxide uses the [fuzzydate](https://docs.rs/fuzzydate/latest/fuzzydate/) library to parse natural language date expressions. This allows you to navigate to daily notes using intuitive phrases.

Some common expressions you can use:

- `today`, `tomorrow`, `yesterday`
- `next monday`, `last friday`
- `2 days ago`, `3 weeks from now`
- `january 15`, `may 4 2025`

<details>
<summary>Click to see more advanced date expression examples</summary>

- Date formats: `7/4/2024`, `2024-12-25`, `3.14.2025`
- Relative dates: `next tuesday`, `last month`
- Duration expressions: `3 days ago`, `2 weeks from now`
- Combined expressions: `2 days after next monday`

</details>

### Editor-Specific Commands

#### Neovim

If you've set up the `:Daily` command in Neovim (as shown in the [setup guide](README#^nvimconfigsetup)), you can navigate with:

- examples ^nvimrelativenamescmds
  - `:Daily two days ago`
  - `:Daily 2 days ago`
  - `:Daily next monday`
  - `:Daily last friday`
  - `:Daily today`
  - `:Daily tomorrow`

#### Other Editors

Markdown Oxide provides LSP workspace commands for daily note navigation. The main command is `jump`, which accepts a natural language date expression as an argument.

For editors with limited LSP command support, these predefined commands are available:

- Predefined Relative Names: ^predefinedNames
  - `today`
  - `tomorrow`
  - `yesterday`
  - `next {monday,tuesday,..., sunday}`
  - `last {monday,tuesday,...}`

## Future Enhancements

We plan to add support for relative navigation between notes, such as "previous daily note" and "next daily note" from the currently open note. You can track this feature in [GitHub issue #101](https://github.com/Feel-ix-343/markdown-oxide/issues/101).
