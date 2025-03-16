use anyhow::{Result, Context};
use connector::Oxide;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::fs::OpenOptions;

use crate::vault::Vault;

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

    let mut oxide: Option<Oxide> = None;
    
    // Log server start
    log_to_file("MCP server started")?;
    
    loop {
        // Read a line directly from stdin
        let mut buffer = String::new();
        log_to_file("Reading from stdin...")?;
        let bytes_read = input.read_line(&mut buffer).context("Failed to read from stdin")?;
        
        if bytes_read == 0 {
            // EOF reached
            log_to_file("EOF reached, exiting")?;
            break;
        }
        
        log_to_file(&format!("Received raw input ({} bytes): {:?}", bytes_read, buffer))?;
        
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
            },
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
                oxide = Some(Oxide::new(&root_dir));
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
            },
            Some("notifications/initialized") => {
                // No response needed for notifications
                log_to_file("Received initialized notification (no response needed)")?;
                continue;
            },
            Some("tools/list") => {
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
                            }
                        ]
                    }
                })
            },
            Some("tools/call") => {
                log_to_file("Handling tools/call request")?;
                let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
                let tool_name = params.get("name").and_then(|n| n.as_str());
                
                log_to_file(&format!("Tool name: {:?}", tool_name))?;
                
                if tool_name == Some("echo") {
                    let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                    let echo_message = arguments.get("message").and_then(|m| m.as_str()).unwrap_or("No message provided");
                    
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
                } else {
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
            },
            Some(unknown) => {
                log_to_file(&format!("Method not found: {}", unknown))?;
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", unknown)
                    }
                })
            },
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
fn create_success_response(id: u64, result: Value) -> String {
    let response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    });
    
    serde_json::to_string(&response).unwrap()
}

/// Create an error response
fn create_error_response(id: u64, code: i32, message: &str, data: Option<Value>) -> String {
    let mut error = json!({
        "code": code,
        "message": message, });
    
    if let Some(data) = data {
        error.as_object_mut().unwrap().insert("data".to_string(), data);
    }
    
    let response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": error
    });
    
    serde_json::to_string(&response).unwrap()
}







mod connector {
    use std::path::{Path, PathBuf};
    
    use anyhow;
    
    use crate::{config::Settings, vault::{Referenceable, Vault}};



    #[derive(Debug)]
    pub struct Oxide {
        vault: Vault
    }

    #[derive(Debug)]
    struct ContextualizedDoc {
        content: String,
        outgoing_links: Vec<LinkedContent>,
        backlinks: Vec<LinkedContent>,
    }

    #[derive(Debug)]
    struct LinkedContent {
        path: PathBuf,
        reference_text: String,
        content: String,
    }

    impl Oxide {
        pub fn new(root_dir: &PathBuf) -> Self {

            let settings = Settings::new(root_dir, true);
            let vault = Vault::construct_vault(&settings, root_dir);


            Self { vault }
        }

        /// Given a document reference, return a contextualized version of the document.
        /// include the full content of the document, the content of outgoing links, and the content of backlinks to the document
        pub fn contextualize_doc(&self, path: &Path) -> Result<ContextualizedDoc, anyhow::Error> {
            // Get the document content
            let rope = self.vault.ropes.get(path)
                .ok_or_else(|| anyhow::anyhow!("Document not found: {:?}", path))?;
            let content = rope.to_string();
            
            // Get outgoing links
            let outgoing_links = self.vault.select_references(Some(path))
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(_, reference)| {
                    // For each reference, find the target document
                    let referenceables = self.vault.select_referenceables_for_reference(reference, path);
                    
                    referenceables.into_iter().next().map(|referenceable| {
                        let target_path = referenceable.get_path();
                        let target_rope = self.vault.ropes.get(target_path)
                            .map(|rope| rope.to_string())
                            .unwrap_or_default();
                        
                        LinkedContent {
                            path: target_path.to_path_buf(),
                            reference_text: reference.data().reference_text.clone(),
                            content: target_rope,
                        }
                    })
                })
                .collect();
                
            // Get backlinks
            let backlinks = self.vault.select_references(None)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(ref_path, reference)| {
                    // Filter references that point to our document
                    if ref_path == path {
                        return None;
                    }
                    
                    // Check if this reference points to our document
                    let path_buf = PathBuf::from(path);
                    let md_file = self.vault.md_files.get(path)?;
                    let referenceable = Referenceable::File(&path_buf, md_file);
                    
                    if referenceable.matches_reference(self.vault.root_dir(), reference, ref_path) {
                        let ref_rope = self.vault.ropes.get(ref_path)
                            .map(|rope| rope.to_string())
                            .unwrap_or_default();
                            
                        Some(LinkedContent {
                            path: ref_path.to_path_buf(),
                            reference_text: reference.data().reference_text.clone(),
                            content: ref_rope,
                        })
                    } else {
                        None
                    }
                })
                .collect();
                
            Ok(ContextualizedDoc {
                content,
                outgoing_links,
                backlinks,
            })
        }
    }


    impl ContextualizedDoc {
        pub fn as_string(&self) -> String {
            let mut result = String::new();
            
            // Add the original document content
            result.push_str("# Original Document\n\n");
            result.push_str(&self.content);
            result.push_str("\n\n");
            
            // Add outgoing links section
            if !self.outgoing_links.is_empty() {
                result.push_str("# Outgoing Links\n\n");
                for link in &self.outgoing_links {
                    result.push_str(&format!("## Link to: {}\n", link.reference_text));
                    result.push_str(&format!("File path: {}\n\n", link.path.display()));
                    
                    // Add a preview of the linked content (first 500 chars or less)
                    let preview = if link.content.len() > 500 {
                        format!("{}...", &link.content[..500])
                    } else {
                        link.content.clone()
                    };
                    
                    result.push_str(&preview);
                    result.push_str("\n\n");
                }
            }
            
            // Add backlinks section
            if !self.backlinks.is_empty() {
                result.push_str("# Backlinks\n\n");
                for link in &self.backlinks {
                    result.push_str(&format!("## Referenced from: {}\n", link.path.display()));
                    result.push_str(&format!("Reference text: {}\n\n", link.reference_text));
                    
                    // Add a preview of the source content (first 500 chars or less)
                    let preview = if link.content.len() > 500 {
                        format!("{}...", &link.content[..500])
                    } else {
                        link.content.clone()
                    };
                    
                    result.push_str(&preview);
                    result.push_str("\n\n");
                }
            }
            
            result
        }
    }






}
