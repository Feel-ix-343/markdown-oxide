use std::path::PathBuf;
use url::Url;

/// Resolve a markdown link `href` relative to the document `base_uri`.
///
/// The previous implementation joined the href directly to the file path,
/// e.g. `/project/README.md` + `subfolder/file.md` → `/project/README.md/subfolder/file.md`,
/// which is incorrect. Relative links must be resolved against the directory
/// containing the source file.
pub fn resolve_link(base_uri: &Url, href: &str) -> Option<PathBuf> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }

    // Ignore external URLs
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("mailto:")
    {
        return None;
    }

    // Strip fragment and query string
    let href = href.split('#').next()?.split('?').next()?;

    let base_path = base_uri.to_file_path().ok()?;
    // Use the parent directory of the source file as the base for relative links
    let base_dir = base_path.parent().unwrap_or(&base_path);
    let resolved = base_dir.join(href);

    Some(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn resolve_from_root_to_subfolder() {
        let base = Url::parse("file:///project/README.md").unwrap();
        let target = resolve_link(&base, "subfolder/file.md").unwrap();
        assert_eq!(target, PathBuf::from("/project/subfolder/file.md"));
    }

    #[test]
    fn resolve_from_subfolder_to_sibling() {
        let base = Url::parse("file:///project/subfolder/a.md").unwrap();
        let target = resolve_link(&base, "b.md").unwrap();
        assert_eq!(target, PathBuf::from("/project/subfolder/b.md"));
    }
}
