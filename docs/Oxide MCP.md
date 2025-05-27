# Oxide MCP

*This documentation was written by Claude (Anthropic's AI assistant) with open thinking.*

## Introduction: Your Knowledge Graph as LLM Context

Markdown-oxide's MCP (Model Context Protocol) integration transforms your personal knowledge management system into a powerful context provider for AI assistants. By bridging your markdown notes with LLMs like Claude, it enables AI to understand not just what you're asking, but the full context of your work, thoughts, and plans.

The key insight: your daily notes are a living record of your current concerns, tasks, and thinking. When combined with the rich backlink structure of your knowledge graph, they provide AI assistants with unprecedented insight into your specific situation.

## The Daily Notes Workflow

Daily notes serve as the primary interface between your thinking and AI assistance. They capture:

- **Current Problems**: What you're actively working on or struggling with
- **Today's Tasks**: Your immediate priorities and action items
- **Reflections**: Thoughts on recent work and experiences
- **Planning**: Future commitments and deadlines

When you invoke an AI assistant through MCP, it can access a temporal window of your daily notes, understanding not just today's context but the evolution of your work over time.

### Example Daily Note Structure
```markdown
# 2024-01-15

## Tasks
- [ ] Review [[Project Alpha]] requirements
- [ ] Prepare for [[Weekly Team Meeting]]
- [x] Submit [[Q1 Budget Proposal]]

## Thoughts
Working through the architecture decisions for [[Project Alpha#Technical Design]].
The main challenge is balancing performance with maintainability...

## Meeting Notes
Discussed timeline concerns with stakeholder. See [[Project Alpha#Risks]] for details.
```

## Extending Context Through Backlinks

The real power emerges when daily notes reference other documents. Each `[[link]]` in your daily notes becomes a pathway for the AI to explore deeper context:

### How Context Expansion Works

1. **Start with Daily Notes**: AI reads your recent daily notes to understand current context
2. **Follow References**: When you mention `[[Project Alpha]]`, the AI can retrieve that document
3. **Explore Backlinks**: The AI sees all other places where Project Alpha is mentioned
4. **Build Complete Picture**: By following these connections, the AI understands the full scope

### Temporal Intelligence

The system prioritizes recent information:
- **Past Daily Notes**: Understand the history of a problem or project
- **Future Daily Notes**: See upcoming deadlines, scheduled tasks, and commitments
- **Recent Modifications**: Backlinks are sorted by modification time, surfacing active areas

## Practical Examples

### Morning Planning
```
You: "What should I focus on today based on my recent work?"

AI (with MCP context):
"Based on your daily notes from the past week, you've been iterating on the
Project Alpha technical design. Yesterday you identified performance concerns
that need addressing. You have a Weekly Team Meeting today where this will
likely come up. Your future daily notes show a deadline on Friday for the
technical specification. I'd suggest prioritizing the performance analysis
this morning so you have concrete data for the team meeting."
```

### Problem Solving
```
You: "I'm stuck on the architecture decision I mentioned"

AI (with MCP context):
"Looking at your notes on Project Alpha's Technical Design, you're weighing
microservices vs monolithic architecture. Your meeting notes from last Tuesday
mention scalability requirements of 10k concurrent users. The linked Performance
Requirements document specifies 200ms response time targets. Based on your
Risk Assessment notes, team expertise is limited with microservices. Given
these constraints, consider a modular monolith approach..."
```

### Task Management
```
You: "What deadlines am I tracking?"

AI (with MCP context):
"From your daily notes and linked documents:
- Friday (3 days): Project Alpha technical specification
- Next Tuesday: Q1 Budget Proposal revisions (mentioned in future daily note)
- Next Friday: Weekly Team Meeting presentation on architecture decision
- End of month: Performance testing completion (referenced in Project Alpha#Timeline)"

Note: This assumes you've created future daily notes with these items, as the system reads actual files rather than parsing dates from content.
```

## Available MCP Tools

### daily_context_range
Retrieves daily notes within a specified date range, providing temporal context about your work and thinking.

**Input Parameters:**
- `past_days`: Number of past days to include (default: 5)
- `future_days`: Number of future days to include (default: 5)

**What it returns:**
- Combined content of daily notes in the range
- Chronologically ordered (oldest to newest)
- Full note content including tasks, reflections, and links

**Use Case Example:**
```json
{
  "name": "daily_context_range",
  "arguments": {
    "past_days": 7,
    "future_days": 7
  }
}
```

This gives the AI a two-week window into your work. The system looks for daily note files matching your configured format (default: `YYYY-MM-DD.md`) in your daily notes folder.

### entity_context
Retrieves comprehensive information about any entity (file, heading, block, or tag) including its definition and all references to it.

**Input Parameters:**
- `ref_id`: Reference identifier (e.g., "Project Alpha", "Project Alpha#Risks", "#important")

**What it returns:**
- Entity definition/content (up to 200 lines for files, 50 lines for sections)
- All backlinks with surrounding context (up to 100 references)
- References sorted by modification time (most recent first)

**Use Case Example:**
```json
{
  "name": "entity_context",
  "arguments": {
    "ref_id": "Project Alpha#Technical Design"
  }
}
```

This provides the AI with deep understanding of specific topics mentioned in your daily notes.

### echo
Simple test tool to verify MCP connectivity.

**Input Parameters:**
- `message`: Text to echo back

## How It Works

### Context Building Process

1. **Entry Point**: Your query triggers the AI to examine recent daily notes
2. **Reference Detection**: The AI identifies all `[[wikilinks]]` and `#tags` in daily notes
3. **Context Expansion**: For important references, the AI retrieves full entity context
4. **Backlink Analysis**: The AI examines where else these concepts appear
5. **Synthesis**: The AI combines this information to understand your situation

### Smart Limits

To provide useful context without overwhelming the AI:
- **File Content**: Up to 200 lines for LLM context mode (vs 14 for hover previews)
- **Section Content**: Up to 50 lines after headings for LLM context mode
- **Backlinks**: Up to 100 references per entity
- **Daily Notes**: Configurable range (default: 5 days past, 5 days future)

These limits are implemented through the `PreviewMode::LlmContext` setting in the codebase.

### Modification Time Priority

References are sorted by file modification time, ensuring the AI sees:
- Active projects and current concerns first
- Historical context when needed
- Stale information deprioritized

## Setup & Configuration

### Enabling MCP Mode

1. **Start markdown-oxide in MCP mode:**
   ```bash
   markdown-oxide mcp --full-dir-path /path/to/your/vault
   ```

2. **Configure daily notes (optional):**
   The system uses these defaults:
   - Daily note format: `%Y-%m-%d` (e.g., 2024-01-15)
   - Daily notes folder: Configurable in your settings

   Configuration can be set through multiple sources including Obsidian's daily note settings if present.

3. **Connect your AI assistant:**
   - The MCP server communicates via stdin/stdout using JSON-RPC
   - Configure your AI assistant to run the markdown-oxide command above
   - The server will automatically watch for file changes in your vault

### Requirements

- Markdown-oxide binary installed and accessible
- Valid vault directory path
- Daily notes following the configured pattern (default: YYYY-MM-DD format)
- MCP-compatible AI assistant that can execute shell commands

### How It Works Under the Hood

- The MCP server reads JSON-RPC messages from stdin and writes responses to stdout
- A file watcher automatically updates the vault index when files change
- The server maintains the vault in memory for fast queries

## Real-World Scenarios

### AI-Powered Tasks and Reminders
"What do I need to do today?"

Your markdown notes become an intelligent task system:
- The AI reads tasks from your daily notes (marked with `- [ ]`)
- Follows links to understand task context and dependencies
- Identifies deadlines mentioned in linked documents
- Finds reminders by checking future daily notes for incoming items

Example: You write in today's note:
```markdown
- [ ] Finish [[API Design]] implementation
- [ ] Review changes for [[Q1 Budget]] (see [[2024-01-20]] for deadline)
```

The AI understands these aren't just tasks—it can follow the links to give you full context about the API design decisions and budget constraints.

### Context-Aware Problem Solving
"I'm stuck on this performance issue"

The AI becomes your debugging partner with full historical context:
- Reads your recent daily notes to understand what you've been working on
- Follows links to technical documentation you've referenced
- Finds similar issues you've solved before by searching your vault
- Understands the specific constraints of your project

Example: When you mention being stuck, the AI already knows from your daily notes that you're working with React, that performance degraded after the recent refactor mentioned three days ago, and that you have a related meeting tomorrow—all without you having to explain.

### Data Extraction from Recent Work
"What were the performance results from this week's experiments?"

The AI extracts actual data from your notes and linked documents:
- Pulls out metrics, numbers, and results you've recorded
- Follows links to detailed experiment logs or data tables
- Aggregates data points scattered across multiple daily notes
- Identifies trends in measurements over time

Example: You've been recording response times in your daily notes:
```markdown
# 2024-01-15
Tested new caching strategy: 145ms average (see [[Performance Tests#Cache Results]])

# 2024-01-16
Without cache: 420ms, With cache: 132ms

# 2024-01-17
After optimization: 98ms! Details in [[Optimization Log]]
```

Later, when you need to use this data, you can ask the LLM to look through your daily notes and aggregate the data.
