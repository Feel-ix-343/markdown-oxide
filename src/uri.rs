use std::path::{Path, PathBuf};
use url::Url;

/// Normalisiert eine URI für Markdown-Dateien, insbesondere bei relativen Pfaden in Unterordnern.
/// Konvertiert file:// URIs zurück in Pfade und löst relative Pfade korrekt auf.
pub fn normalize_markdown_uri(uri: &str, workspace_root: &Path) -> Option<PathBuf> {
    // 1. URI in Pfad umwandeln (file:// -> absoluter Pfad)
    let path = if let Ok(url) = Url::parse(uri) {
        if url.scheme() == "file" {
            // Konvertiere URL-Pfad in System-Pfad
            let mut path = PathBuf::from(url.to_file_path().ok()?);

            // 2. Relative Pfade auflösen (z.B. "../subfolder/file.md")
            if path.is_relative() {
                path = workspace_root.join(path);
            }

            path
        } else {
            // Nicht-file URIs ignorieren
            return None;
        }
    } else {
        // Keine URI - direkt als Pfad behandeln
        PathBuf::from(uri)
    };

    // 3. Pfad normalisieren (.. und . entfernen)
    dunce::canonicalize(path).ok()
}