use crate::uri::normalize_markdown_uri;
use std::path::PathBuf;

// ... (bestehende Imports)

/// Handler für die TextDocument/DidOpen Notification
pub fn handle_did_open(
    state: &mut ServerState,
    params: DidOpenTextDocumentParams,
) -> Result<()> {
    let uri = params.text_document.uri;
    let path = normalize_markdown_uri(&uri.to_string(), &state.workspace_root)
        .ok_or_else(|| Error::invalid_params("Invalid file URI"))?;

    // ... (bestehender Code)
}

/// Handler für die TextDocument/DidChange Notification
pub fn handle_did_change(
    state: &mut ServerState,
    params: DidChangeTextDocumentParams,
) -> Result<()> {
    let uri = params.text_document.uri;
    let path = normalize_markdown_uri(&uri.to_string(), &state.workspace_root)
        .ok_or_else(|| Error::invalid_params("Invalid file URI"))?;

    // ... (bestehender Code)
}

/// Neue Funktion zur Auflösung von Markdown-Links in Dokumenten
pub fn resolve_markdown_link(
    state: &ServerState,
    link_path: &str,
    current_file_path: &Path,
) -> Option<PathBuf> {
    // 1. Link-Pfad normalisieren
    let link_path = PathBuf::from(link_path);

    // 2. Relativen Pfad vom aktuellen Dokument aus auflösen
    let resolved_path = if link_path.is_relative() {
        current_file_path.parent()?.join(link_path)
    } else {
        link_path
    };

    // 3. Pfad normalisieren und auf Existenz prüfen
    dunce::canonicalize(resolved_path).ok().filter(|p| p.exists())
}