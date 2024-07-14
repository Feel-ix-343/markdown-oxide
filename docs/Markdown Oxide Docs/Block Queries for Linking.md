

When linking to a [block](Block), enter a query with specific filters to easily find the block that you want. 

## The Problem

Perhaps the hardest part of block linking is finding the block that you want to link to. [Markdown Oxide v0 block completions](<Block Completions>) allow users to do a full-text search, but this rarely gets us the results we want

For example:

- You want to link to a block that was in yesterday's note, but current markdown oxide does not allow you to search the text of only yesterday's note, so you have to search all text in your vault, and you cannot find what you want. 
- While researching, you write down your thoughts in a list, but when it is time to outline your writing, you struggle to find these thoughts. You know you wrote something down, you want to link to it, but you cannot find it!



## Block Queries as the solution

Block Queries allow you to link to the blocks that you want by first specifying filters, then entering a text search. You do this by entering a query (with markdown-link style syntax) in your notes document, and results are presented through LSP completions.     ^4b1be

### Filters and syntax

The query syntax is quite simple. Declare the query with an empty Markdown Link or Wiki link: `[[{query}` or `[{display}]({query}` where `query` is the query and `display`, if you are using markdown links, any display text will remain unchanged. 

Here are the filters[^1]. The section in code blocks replaces `{query}` above. `{search}` is the search string that the selected block will be fuzzy matched by. 

- [ ] File: filter blocks by which file they are contained in
    - [ ] Current file: `# {search}`. Note the space
    - [ ] File name: `{filename}# {search}`
    - [ ] Daily note syntax: `{daily-note name}# {search}` where daily-note name is the [[Daily Notes#Completion Names|daily-note relative name]]
- [ ] Outgoing references: filter blocks by outgoing references they have or do not have: `out:{refname}`
- [ ] ...



### Query syntax


How will this work? You will try 


[^1]: ![[Documentation Notes#^workinprogress]]
