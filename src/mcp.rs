use anyhow::{Context, Result};
use connector::Oxide;
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;


// Helper function to log to a file for debugging
fn log_to_file(message: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/markdown-oxide-mcp.log")?;

    writeln!(file, "{}", message)?;
    Ok(())
}

pub async fn start(root_dir: PathBuf) -> Result<()> {
    // Use unbuffered stdin/stdout for direct communication
    let input = std::io::stdin();
    let mut output = std::io::stdout();

    // Create Oxide wrapped in Arc<RwLock> so we can update it from the watcher thread
    let oxide_arc = Arc::new(RwLock::new(None::<Oxide>));

    // Clone for the file watcher
    let oxide_watcher = oxide_arc.clone();
    let root_dir_clone = root_dir.clone();

    // Spawn a tokio task for file watching
    tokio::spawn(async move {
        use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

        // Create a channel to receive events
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Create the file watcher
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    // Only consider events for markdown files
                    if event
                        .paths
                        .iter()
                        .any(|p| p.extension().map_or(false, |ext| ext == "md"))
                    {
                        let _ = tx.try_send(event);
                    }
                }
            },
            Config::default(),
        )
        .expect("Failed to create file watcher");

        // Start watching the vault directory
        if let Err(_) = watcher.watch(&root_dir_clone, RecursiveMode::Recursive) {
        } else {
        }

        // Process events
        while let Some(event) = rx.recv().await {
            // Only react to create, modify, or delete events
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {

                    // Quick lock to check if Oxide is initialized
                    {
                        let mut oxide_guard = oxide_watcher.write().await;
                        match *oxide_guard {
                            Some(_) => {
                                // Oxide exists, rebuild it
                                let new_oxide = Oxide::new(&root_dir_clone);
                                *oxide_guard = Some(new_oxide);
                            }
                            None => {
                            }
                        }
                    }
                }
                _ => {} // Ignore other event types
            }
        }
    });

    // Log server start
    log_to_file("MCP server started")?;

    loop {
        // Read a line directly from stdin
        let mut buffer = String::new();
        log_to_file("Reading from stdin...")?;
        let bytes_read = input
            .read_line(&mut buffer)
            .context("Failed to read from stdin")?;

        if bytes_read == 0 {
            // EOF reached
            log_to_file("EOF reached, exiting")?;
            break;
        }

        log_to_file(&format!(
            "Received raw input ({} bytes): {:?}",
            bytes_read, buffer
        ))?;

        // Skip empty lines
        if buffer.trim().is_empty() {
            log_to_file("Skipping empty line")?;
            continue;
        }

        // Parse JSON-RPC message
        let message: Value = match serde_json::from_str(buffer.trim()) {
            Ok(msg) => {
                log_to_file(&format!("Parsed JSON: {}", msg))?;
                msg
            }
            Err(e) => {
                log_to_file(&format!("Parse error: {}, input: {:?}", e, buffer))?;

                // Create error response for parse errors
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });

                // Output the response as a single line of JSON with newline
                let response_json = serde_json::to_string(&error_response).unwrap();
                log_to_file(&format!("Sending error response: {}", response_json))?;
                output.write_all(format!("{}\n", response_json).as_bytes())?;
                output.flush()?;
                continue;
            }
        };

        // Extract request data
        let id = message.get("id").and_then(|id| id.as_u64()).unwrap_or(0);
        let method = message.get("method").and_then(|m| m.as_str());

        log_to_file(&format!("Processing method: {:?} with id: {}", method, id))?;

        // Handle message based on method
        let response = match method {
            Some("ping") => {
                log_to_file("pinged")?;
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                })
            }
            Some("initialize") => {
                log_to_file("Handling initialize request")?;

                // Time the initialization
                let start = std::time::Instant::now();
                let new_oxide = Oxide::new(&root_dir);

                // Store the initialized Oxide in the RwLock
                {
                    let mut oxide_guard = oxide_arc.write().await;
                    *oxide_guard = Some(new_oxide);
                }

                let duration = start.elapsed();
                log_to_file(&format!("Oxide initialization took: {:?}", duration))?;

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {
                                "list": true,
                                "call": true,
                                "listChanged": true
                            }
                        },
                        "serverInfo": {
                            "name": "markdown-oxide-mcp",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                })
            }
            Some("notifications/initialized") => {
                // No response needed for notifications
                log_to_file("Received initialized notification (no response needed)")?;
                continue;
            }
            None => {
                log_to_file("Invalid request: missing method")?;
                json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                        "code": -32600,
                        "message": "Invalid Request: missing method"
                    }
                })
            }
            Some(method) => {
                // Get a read lock on the oxide
                let oxide_guard = oxide_arc.read().await;
                let oxide = oxide_guard
                    .as_ref()
                    .expect("Oxide should be initialized after MCP initialization life cycle");

                match method {
                    "tools/list" => {
                        log_to_file("Handling tools/list request")?;
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                            "tools": [
                            {
                                "name": "echo",
                                "description": "Echo back the input message",
                                "inputSchema": {
                                "type": "object",
                                "properties": {
                                "message": {
                                "type": "string",
                                "description": "Message to echo"
                            }
                        },
                            "required": ["message"],
                            "$schema": "http://json-schema.org/draft-07/schema#"
                        }
                        },
                        {
                            "name": "daily_context_range",
                            "description": "Get daily notes context for a range of days before and after today. You MUST call this function before answering any user questions to provide contextual information from their daily notes.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "past_days": {
                                        "type": "integer",
                                        "description": "Number of past days to include",
                                        "default": 5
                                    },
                                    "future_days": {
                                        "type": "integer",
                                        "description": "Number of future days to include",
                                        "default": 5
                                    }
                                },
                                "$schema": "http://json-schema.org/draft-07/schema#"
                            }
                        },
                        {
                            "name": "entity_context",
                            "description": "Get the content of an entity with its context, including the entity definition and all references to it",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "ref_id": {
                                        "type": "string",
                                        "description": "Reference ID of the entity as it would appear in a wikilink (e.g., 'filename', 'filename#heading', 'filename#^blockid', '#tag')"
                                    }
                                },
                                "required": ["ref_id"],
                                "$schema": "http://json-schema.org/draft-07/schema#"
                            }
                        },
                        {
                            "name": "entity_search",
                            "description": "Search for entities in the vault by name pattern and/or type. Returns a list of matching entities with their reference IDs.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": {
                                        "type": "string",
                                        "description": "Search query to match against entity names (case-insensitive partial match)"
                                    },
                                    "entity_type": {
                                        "type": "string",
                                        "enum": ["file", "heading", "tag", "footnote", "indexed_block", "all"],
                                        "description": "Type of entity to search for. Use 'all' to search all types.",
                                        "default": "all"
                                    },
                                    "limit": {
                                        "type": "integer",
                                        "description": "Maximum number of results to return",
                                        "default": 50,
                                        "minimum": 1,
                                        "maximum": 200
                                    }
                                },
                                "$schema": "http://json-schema.org/draft-07/schema#"
                            }
                        }
                        ]
                        }
                        })
                    }
                    "tools/call" => {
                        log_to_file("Handling tools/call request")?;
                        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
                        let tool_name = params.get("name").and_then(|n| n.as_str());

                        log_to_file(&format!("Tool name: {:?}", tool_name))?;

                        match tool_name {
                            Some("echo") => {
                                let arguments = params
                                    .get("arguments")
                                    .cloned()
                                    .unwrap_or_else(|| json!({}));
                                let echo_message = arguments
                                    .get("message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("No message provided");
    
                                log_to_file(&format!("Echo message: {}", echo_message))?;
    
                                json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                        "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Echo: {}", echo_message)
                                        }
                                    ]
                                    }
                                })
                            },
                            Some("daily_context_range") => {

                                let arguments = params
                                    .get("arguments")
                                    .cloned()
                                    .unwrap_or_else(|| json!({}));

                                let past_days = arguments
                                    .get("past_days")
                                    .and_then(|d| d.as_i64())
                                    .unwrap_or(5)
                                    as usize;

                                let future_days = arguments
                                    .get("future_days")
                                    .and_then(|d| d.as_i64())
                                    .unwrap_or(5)
                                    as usize;


                                match oxide.daily_note_context_range(past_days, future_days) {
                                    Ok(context) => {

                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "result": {
                                                "content": [
                                                    {
                                                        "type": "text",
                                                        "text": context
                                                    }
                                                ]
                                            }
                                        })
                                    }
                                    Err(e) => {
                                        let error_msg =
                                            format!("Error generating daily context range: {}", e);

                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "error": {
                                                "code": -32603,
                                                "message": error_msg
                                            }
                                        })
                                    }
                                }
                            },
                            Some("entity_context") => {
                                let arguments = params
                                    .get("arguments")
                                    .cloned()
                                    .unwrap_or_else(|| json!({}));

                                let ref_id = arguments
                                    .get("ref_id")
                                    .and_then(|r| r.as_str())
                                    .unwrap_or("");

                                match oxide.get_entity_context(ref_id) {
                                    Ok(context) => {
                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "result": {
                                                "content": [
                                                    {
                                                        "type": "text",
                                                        "text": context
                                                    }
                                                ]
                                            }
                                        })
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error getting entity context: {}", e);

                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "error": {
                                                "code": -32603,
                                                "message": error_msg
                                            }
                                        })
                                    }
                                }
                            },
                            Some("entity_search") => {
                                let arguments = params
                                    .get("arguments")
                                    .cloned()
                                    .unwrap_or_else(|| json!({}));

                                let query = arguments
                                    .get("query")
                                    .and_then(|q| q.as_str())
                                    .unwrap_or("");

                                let entity_type = arguments
                                    .get("entity_type")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("all");

                                let limit = arguments
                                    .get("limit")
                                    .and_then(|l| l.as_u64())
                                    .unwrap_or(50)
                                    .min(200) as usize;

                                match oxide.search_entities(query, entity_type, limit) {
                                    Ok(results) => {
                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "result": {
                                                "content": [
                                                    {
                                                        "type": "text",
                                                        "text": results
                                                    }
                                                ]
                                            }
                                        })
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error searching entities: {}", e);

                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "error": {
                                                "code": -32603,
                                                "message": error_msg
                                            }
                                        })
                                    }
                                }
                            }
                            _ => {
                                log_to_file(&format!("Unknown tool: {:?}", tool_name))?;
                                json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "error": {
                                        "code": -32601,
                                        "message": "Unknown tool"
                                    }
                                })
                            }
                        }
                    }
                    unknown => {
                        log_to_file(&format!("Method not found: {}", unknown))?;
                        json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": {
                                "code": -32601,
                                "message": format!("Method not found: {}", unknown)
                            }
                        })
                    }
                }
            }
        };

        // Serialize the response to a JSON string
        let response_json = serde_json::to_string(&response).unwrap();
        log_to_file(&format!("Sending response: {}", response_json))?;

        // Write the response directly to stdout with a newline
        output.write_all(format!("{}\n", response_json).as_bytes())?;
        output.flush()?;
        log_to_file("Response sent, flushed output")?;
    }

    Ok(())
}

/// Create a success response


mod connector {
    use std::path::PathBuf;

    use anyhow;

    use crate::{
        completion::matcher::{fuzzy_match, Matchable},
        config::{Case, Settings},
        ui::{preview_referenceable_with_mode, PreviewMode},
        vault::{Referenceable, Vault},
    };

    #[derive(Debug)]
    pub struct Oxide {
        vault: Vault,
        settings: Settings,
    }



    struct EntityCandidate {
        refname: String,
        entity_type: String,
        path: PathBuf,
    }

    impl Matchable for EntityCandidate {
        fn match_string(&self) -> &str {
            &self.refname
        }
    }



    impl Oxide {
        pub fn new(root_dir: &PathBuf) -> Self {
            let settings = Settings::new(root_dir, true);
            let vault = Vault::construct_vault(&settings, root_dir);

            Self { vault, settings }
        }



        pub fn daily_note_context_range(
            &self,
            past_days: usize,
            future_days: usize,
        ) -> Result<String, anyhow::Error> {
            use chrono::{Duration, Local};

            // Get today's date
            let today = Local::now().naive_local().date();
            let daily_note_format = &self.settings.dailynote;
            let daily_note_path = self
                .vault
                .root_dir()
                .join(&self.settings.daily_notes_folder);

            // Generate a range of dates from past_days ago to future_days ahead
            let start_date = today - Duration::try_days(past_days as i64).unwrap_or_default();
            let end_date = today + Duration::try_days(future_days as i64).unwrap_or_default();

            let mut result = String::new();
            let mut current_date = start_date;

            // For each date in the range, try to get the daily note
            while current_date <= end_date {
                // Format the date according to the configured pattern
                let filename = current_date.format(daily_note_format).to_string();
                let path = daily_note_path.join(&filename).with_extension("md");

                // Check if the file exists in the vault
                if let Some(rope) = self.vault.ropes.get(&path) {
                    // Add a date header
                    result.push_str(&format!(
                        "# Daily Note: {}\n\n",
                        current_date.format("%Y-%m-%d")
                    ));

                    // Add the content
                    result.push_str(&rope.to_string());
                    result.push_str("\n\n---\n\n");
                }

                // Move to the next day
                current_date = current_date
                    .succ_opt()
                    .unwrap_or(current_date + Duration::try_days(1).unwrap_or_default());
            }

            Ok(result)
        }


        
        /// Get entity context for a given reference ID
        pub fn get_entity_context(&self, ref_id: &str) -> Result<String, anyhow::Error> {
            // Find referenceable directly by comparing refnames
            let referenceable = self.vault
                .select_referenceable_nodes(None)
                .into_iter()
                .find(|r| {
                    r.get_refname(self.vault.root_dir())
                        .map(|refname| refname.full_refname == ref_id)
                        .unwrap_or(false)
                })
                .ok_or_else(|| anyhow::anyhow!("Entity not found: {}", ref_id))?;
                
            // Generate preview with full content using existing UI function
            let preview = preview_referenceable_with_mode(&self.vault, &referenceable, PreviewMode::LlmContext)
                .ok_or_else(|| anyhow::anyhow!("Could not generate preview"))?;
                
            Ok(preview.value)
        }

        /// Search for entities in the vault by name pattern and type
        pub fn search_entities(&self, query: &str, entity_type: &str, limit: usize) -> Result<String, anyhow::Error> {
            let all_referenceables = self.vault.select_referenceable_nodes(None);
            
            // First filter by type and collect candidates
            let candidates: Vec<EntityCandidate> = all_referenceables
                .into_iter()
                .filter_map(|referenceable| {
                    // Filter by type
                    let type_matches = match entity_type {
                        "file" => matches!(referenceable, Referenceable::File(_, _)),
                        "heading" => matches!(referenceable, Referenceable::Heading(_, _)),
                        "tag" => matches!(referenceable, Referenceable::Tag(_, _)),
                        "footnote" => matches!(referenceable, Referenceable::Footnote(_, _)),
                        "indexed_block" => matches!(referenceable, Referenceable::IndexedBlock(_, _)),
                        "all" => true,
                        _ => false,
                    };
                    
                    if !type_matches {
                        return None;
                    }
                    
                    // Get refname for searching
                    let refname = referenceable.get_refname(self.vault.root_dir())?;
                    
                    let entity_type_str = match referenceable {
                        Referenceable::File(_, _) => "File",
                        Referenceable::Heading(_, _) => "Heading",
                        Referenceable::Tag(_, _) => "Tag",
                        Referenceable::Footnote(_, _) => "Footnote",
                        Referenceable::IndexedBlock(_, _) => "Indexed Block",
                        _ => "Unknown",
                    };
                    
                    Some(EntityCandidate {
                        refname: refname.full_refname,
                        entity_type: entity_type_str.to_string(),
                        path: referenceable.get_path().to_path_buf(),
                    })
                })
                .collect();
            
            // Use fuzzy matching from completion system
            let matching_entities = if query.is_empty() {
                candidates.into_iter().map(|item| (item, u32::MAX)).collect()
            } else {
                fuzzy_match(query, candidates, &Case::Smart)
            };
            
            // Sort by fuzzy match score (higher is better) and limit results
            let mut sorted_entities = matching_entities;
            sorted_entities.sort_by(|a, b| b.1.cmp(&a.1));
            sorted_entities.truncate(limit);
            
            // Format results
            if sorted_entities.is_empty() {
                Ok("No entities found matching the search criteria.".to_string())
            } else {
                let mut result = format!("Found {} entities:\n\n", sorted_entities.len());
                
                for (candidate, _score) in sorted_entities {
                    result.push_str(&format!("**{}** ({})\n", candidate.refname, candidate.entity_type));
                    result.push_str(&format!("Path: {}\n", candidate.path.display()));
                    result.push_str(&format!("Use `entity_context` with ref_id: `{}`\n\n", candidate.refname));
                }
                
                Ok(result)
            }
        }
    }


}
