// Aktualisierte Version der Referenzerkennung im Parser
// Fokus: Nur gültige [[...]]-Links oder Code-Blöcke als Referenzen markieren

fn parse_reference(&mut self, start: usize) -> Option<SyntaxNode> {
    // Prüfen, ob wir uns in einem Code-Block oder einem Link befinden
    let in_code_block = self.current_node_is(CodeBlock);
    let in_link = self.current_node_is(Link);

    if !in_code_block && !in_link {
        return None;
    }

    // Restliche Logik für gültige Referenzen beibehalten
    let end = self.find_closing_brackets(start)?;
    if end <= start + 2 {
        return None; // Leere Referenz [[ ]]
    }

    let content = &self.input[start + 2..end - 2];
    if content.trim().is_empty() {
        return None;
    }

    Some(self.create_reference_node(start, end, content))
}

fn current_node_is(&self, kind: SyntaxKind) -> bool {
    self.current_node()
        .map(|node| node.kind() == kind)
        .unwrap_or(false)
}