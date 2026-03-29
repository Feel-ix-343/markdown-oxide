use tower_lsp::lsp_types::{
    DocumentSymbol, DocumentSymbolResponse, SymbolKind, WorkspaceSymbol, WorkspaceSymbolResponse,
};
use url::Url;

use crate::vault::{Referenceable, Vault};

pub fn document_symbols(vault: &Vault, path: &std::path::Path) -> Option<DocumentSymbolResponse> {
    let file = vault.md_files.get(path)?;
    let mut symbols = Vec::new();

    // Add headings as symbols
    for heading in &file.headings {
        symbols.push(DocumentSymbol {
            name: heading.heading.clone(),
            detail: None,
            kind: SymbolKind::STRING,
            tags: None,
            deprecated: None,
            range: heading.range,
            selection_range: heading.range,
            children: None,
        });
    }

    Some(DocumentSymbolResponse::Nested(symbols))
}

pub fn workspace_symbol(vault: &Vault, query: &str) -> Option<WorkspaceSymbolResponse> {
    let mut symbols = Vec::new();

    // Search through all referenceables
    for referenceable in vault.select_referenceable_nodes(None) {
        if referenceable.refname().to_lowercase().contains(&query.to_lowercase()) {
            let location = match referenceable {
                Referenceable::File(path, file) => {
                    let url = Url::from_file_path(path).ok()?;
                    url.with_path(&file.name)
                }
                Referenceable::Heading(path, heading) => {
                    let url = Url::from_file_path(path).ok()?;
                    url.with_path(&format!("{}#{}", file.name, heading.slug))
                }
                Referenceable::Tag(path, tag) => {
                    let url = Url::from_file_path(path).ok()?;
                    url.with_path(&format!("#{}", tag.tag_ref))
                }
                Referenceable::Footnote(path, footnote) => {
                    let url = Url::from_file_path(path).ok()?;
                    url.with_path(&format!("{}#^{}", file.name, footnote.index))
                }
                Referenceable::IndexedBlock(path, block) => {
                    let url = Url::from_file_path(path).ok()?;
                    url.with_path(&format!("{}#^{}", file.name, block.index))
                }
                Referenceable::Alias(path, alias, original) => {
                    let url = Url::from_file_path(path).ok()?;
                    url.with_path(original)
                }
                Referenceable::UnresolvedHeading(_, _) => continue,
                Referenceable::UnresolvedFile(_) => continue,
                Referenceable::UnresolvedTag(_) => continue,
            };

            symbols.push(WorkspaceSymbol {
                name: referenceable.refname().to_string(),
                kind: match referenceable {
                    Referenceable::File(_, _) => SymbolKind::FILE,
                    Referenceable::Heading(_, _) => SymbolKind::STRING,
                    Referenceable::Tag(_, _) => SymbolKind::KEYWORD,
                    Referenceable::Footnote(_, _) => SymbolKind::REFERENCE,
                    Referenceable::IndexedBlock(_, _) => SymbolKind::REFERENCE,
                    Referenceable::Alias(_, _, _) => SymbolKind::STRING,
                    Referenceable::UnresolvedHeading(_, _) => SymbolKind::STRING,
                    Referenceable::UnresolvedFile(_) => SymbolKind::FILE,
                    Referenceable::UnresolvedTag(_) => SymbolKind::KEYWORD,
                },
                tags: None,
                container_name: None,
                location: Location::new(location, Range::default()),
                data: None,
            });
        }
    }

    if symbols.is_empty() {
        None
    } else {
        Some(WorkspaceSymbolResponse::Nested(symbols))
    }
}