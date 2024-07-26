
> [!warning]
> Work in progress documentation

# Markdown Oxide v1 Block Organization

Methods of organizing blocks:

- labeling
- Indexing
- Grouping
- Relating to other blocks

And we take these actions situationally in order to make the blocks *usable* -- meaning that we can find and work with them later.

## Labeling Blocks

Label a block or blocks as belonging to a [collection](2024-07-26#^collection). This both collects the block into the collection and allows for filtering blocks by collection label in block queries.

In this system, labeling is done by markdown links or tags. Both have the same effect, but different practical uses. Links can be used to label the block while writing it, while tags can be used to label the block after the fact. Additionally, tags allows for automation in labeling blocks. 

* Links: use markdown to query collections then insert a link to the collection, choosing either to display the link as collection name text or a custom display name. 
* Tagging: use markdown to query collections, then insert the tag. 
    + manual: this tag can be inserted wherever you choose, consider that this choice may affect how the block is rendered and mirrored. 
    + automated: a command can be selected to append a tag to the end of a block
        + suggests based on matching of words in the block to collection names
        + suggests based on the properties (labels) of a parent block.
        + for multiple sub-blocks, suggests labeling sub-blocks by the labels of a parent block through tags. (labeling sub-blocks of a task as tasks, for example)


> [!note]- Choosing not to use inheritance
> You may have noticed that in regard to hierarchy for these features, we chose a tempalting style over an inheritance style. The cost of this is both additional syntax and additional actions: sub-blocks could inherit all lables of their parent block without any work on the user's part. However, this reduces user control, which is a major cost. Consider these examples: we have a task block with sub-blocks which are sub-tasks; inheritance convieniently labels these sub-blocks as tasks, which is correct and useful later. However, in both cases of *the sub-blocks are not sub-tasks* and *only a few of the sub-block are sub-tasks*, inheritance is improper. Our solution adds a minimal step to ensure correctness in our representation: you are able to label all sub-blocks with the properties of the parent block: allowing us to label all sub-blocks as sub-tasks in one quick action, and we are also able to manually label sub-blocks as sub-tasks, allowing on the sub-blocks that really are sub-tasks to be recorded as sub-tasks. 
> 
> Generally, we reap the benefits of a normalized database, in which each block, no matter its relation to other blocks, is treated on the same level -- ensuring complete correctness.

### Examples

- tasks
- taking notes on a topic

## Indexing Blocks

- Daily notes: index many blocks where applicable by date

## Grouping Blocks

- Write blocks in designated files to group them by the file-name. 
    * note that, conceptually, these files are not considered collections. More just grouped thought-dumps

### Examples

- Notes for a specific project.

## Relating Blocks to eachother

* generally relating single blocks to single blocks:
    + through block linking: operate a block query, select a block, insert a link to the block. 
* subordinating many blocks to single block:
    + from definition: structuraly mark a block as a sub-block through making it a sub-list item
    + through mirror-blocks: if you are not in the same file as the block you are subordinating to, insert a *mirror-block*, then mark blocks as subordinate the mirrored block by making them sublist items
        + a mirror-block is a list item block containing only a block-embed and an optional colon as the last character
* remixing many blocks in single block: one block serves as the parial content of another block to be written. Create a remix block by embedding a block in another one. 


### Examples

- Tasks, and sub-takss
- Takss, and task-details
- Argument and supporting points
