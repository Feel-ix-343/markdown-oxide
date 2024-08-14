#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct EntityFileLocation {
    pub(crate) line: Line,
}

pub type Line = usize;
pub type Lines = std::ops::Range<Line>;

impl EntityFileLocation {
    pub(crate) fn from_range(range: tree_sitter::Range) -> EntityFileLocation {
        EntityFileLocation {
            line: range.start_point.row,
        }
    }
}
