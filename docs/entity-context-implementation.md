# Entity Context Implementation

## Overview

The entity context tool provides comprehensive information about entities in your markdown vault by leveraging existing vault APIs and UI preview functions. It returns both the entity definition and all references to that entity with surrounding context.

## Implementation Details

### Key Components Added

#### 1. PreviewMode Enum
Located in `src/ui.rs`:
```rust
#[derive(Copy, Clone)]
pub enum PreviewMode {
    Hover,     // Limited content: 20 backlinks, 14 lines for files
    LlmContext // Expanded content: 100 backlinks, 200 lines for files
}
```

#### 2. Enhanced UI Functions
Added new variants that accept `PreviewMode`:
- `preview_reference_with_mode()`
- `preview_referenceable_with_mode()`
- `referenceable_string_with_mode()`

Original functions remain unchanged and delegate to new ones with `PreviewMode::Hover`.

#### 3. Vault Preview Function
Added `select_referenceable_preview_with_mode()` to `Vault` that respects content limits based on mode:
- **Hover mode**: 14 lines for files, 10 lines after headings
- **LLM context mode**: 200 lines for files, 50 lines after headings

#### 4. MCP Entity Context Method
```rust
pub fn get_entity_context(&self, ref_id: &str) -> Result<String, anyhow::Error>
```

### How It Works

1. **Reference Resolution**: Finds referenceables by matching their refname against the provided `ref_id`
2. **Content Generation**: Uses `preview_referenceable_with_mode()` with `PreviewMode::LlmContext`
3. **Backlink Handling**: Automatically includes up to 100 backlinks (sorted by modification time)

### Supported Reference ID Formats

| Entity Type | Reference ID Format | Example |
|-------------|-------------------|---------|
| File | `filename` | `"project-notes"` |
| Heading | `filename#heading` | `"project-notes#Overview"` |
| Block | `filename#^blockid` | `"project-notes#^important"` |
| Tag | `#tagname` | `"#todo"` |

## Usage Example

```json
{
  "name": "entity_context",
  "arguments": {
    "ref_id": "architecture#Design Principles"
  }
}
```

This returns:
- The heading "Design Principles" from the "architecture" file
- Up to 50 lines of content after the heading
- Up to 100 backlinks showing where this heading is referenced
- Each backlink includes the full line containing the reference

## Benefits

1. **Code Reuse**: Leverages existing vault APIs and UI functions
2. **No Breaking Changes**: Original functions remain unchanged
3. **Consistent Behavior**: Uses same reference resolution as hover/completion
4. **Smart Sorting**: Backlinks sorted by modification time (most recent first)
5. **Appropriate Limits**: Provides extensive context without overwhelming

## Technical Notes

- The implementation avoids constructing `Reference` objects with dummy ranges
- Instead, it directly matches referenceables by their canonical refname
- This approach is cleaner and avoids issues with range dependencies in the LSP code