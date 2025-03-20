# Block Queries for Enhanced Linking

One of Markdown Oxide's powerful features is the ability to link directly to specific blocks (paragraphs, list items) within your notes. Block queries make this process more efficient by providing targeted filtering.

## The Challenge of Block Discovery

Finding the exact block you want to link to can be challenging in a large knowledge base. Standard block completions offer full-text search across your vault, but this approach has limitations:

- **Daily note overload**: When you want to link to content in yesterday's note, searching across all notes makes finding it difficult
- **Research organization**: As you collect thoughts in various notes, retrieving specific ideas can be like finding a needle in a haystack
- **Context specificity**: Sometimes you need to find blocks related to specific topics or within certain documents

## How Block Queries Solve This

Block queries provide a targeted approach to finding and linking blocks by allowing you to apply filters before performing a text search. This powerful feature surfaces relevant blocks through your editor's LSP completions. ^4b1be

## Query Syntax and Usage

To use block queries, start typing a Markdown or Wiki link with specific query patterns:

```markdown
[[{query}]]    # Wiki-link format
[{display}]({query})    # Markdown link format
```

Where:

- `{query}` is your filter expression
- `{display}` (optional in Markdown links) is display text that remains unchanged when the completion is selected

### Available Filters

The following filters can be used in your queries:

| Filter Type         | Syntax                   | Description                                                     |
| ------------------- | ------------------------ | --------------------------------------------------------------- |
| Current file        | `# {search}`             | Search only in the current file                                 |
| Specific file       | `{filename}# {search}`   | Search in a specific file                                       |
| Daily note          | `{daily-note}# {search}` | Search in a daily note using [relative names](Daily%20Notes.md) |
| Outgoing references | `out:{refname}`          | Filter blocks that link to specific references                  |

### Examples

```markdown
[[# meeting notes]]    # Find blocks containing "meeting notes" in current file
[[yesterday# action items]]    # Find "action items" in yesterday's daily note
```

> [!note] Development Status
> Block queries functionality is still under development. While the basic block structure and linking are fully implemented (using the `^blockid` syntax), the advanced query filtering described here represents planned functionality.
>
> Currently, block links work with the following syntax:
>
> - `[[file#^blockid]]` (Wiki-style links)
> - `[display text](file#^blockid)` (Markdown links)

## Implementation Details

Markdown Oxide identifies blocks in your notes and allows you to reference them:

1. **Block ID**: Each block can have a unique ID preceded by a caret (`^`), such as `^ab12c`
2. **Auto-ID Generation**: When you reference an unindexed block, the system can automatically generate a random ID
3. **Block Navigation**: You can navigate to blocks using "go to definition" in your editor

## Future Enhancements

This feature is under active development. Future updates will include:

- Tag filters to find blocks with specific tags
- Date range filters for temporal queries
- Combining multiple filters for complex queries
