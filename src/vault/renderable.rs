use std::ops::Range;

pub use super::parsing::md_nodes::CalloutType;

type RenderableSpan = Range<u8>;

/// Example: We are rendering a file with backlinks and h1 headings with backlinks 
///
/// - Top level: List of renderables for the top level paragraphs, headings, and backlinks
/// - Heading: Each heading has children of the paragraphs in the heading and the heading's backlinks
///
/// Where do the backlinks to the file go? You decide; either in the file's top level content or below it
pub struct Renderable<'fs> {
    defined_renderable: RenderableType<'fs>,
    backlinks_renderable: ReferencedRenderable<'fs>,
    children: Option<Vec<Renderable<'fs>>>
}

pub enum RenderableType<'fs> {
    Referenced(ReferencedRenderable<'fs>),
    Defined(RenderableItem<'fs>)
}

pub struct ReferencedRenderable<'fs> {
}

/// Data enum representing supported UI-renderable markdown elements.
///
/// Note that there are no file ranges; The structure and positioning of the final rendered
/// view is to be decided by the UI renderer
///
/// Making Renderable an enum implies that the rendering for each item should be implemented manually
/// This is intended
pub enum RenderableItem<'fs> {
    Block {
        text: Vec<Text<'fs>>,
    },
    Heading {
        text: Text<'fs>,
        level: u8,
    },
    OrderedListItemIdentifier {
        level: u8
    },
    UnorderedListItemIdentifier {
        level: u8
    },
    TaskItem {
        checked: bool,
        text: Vec<Text<'fs>>,
    },
    ListItem {
        text: Vec<Text<'fs>>,
    },
    Callout {
        callout_type: CalloutType,
        title: Text<'fs>,
        description: Vec<Text<'fs>>,
    }
}

/// Represents on line of Text
pub struct Text<'fs> {
    text: &'fs str,
    references: Vec<RenderableLink<'fs>>,
    tags: Vec<RenderableTag<'fs>>
}

pub struct RenderableTag<'fs> {
    full_span: RenderableSpan,
    tags: Vec<&'fs str>
}

pub struct RenderableLink<'fs> {
    pub full_span: RenderableSpan,
    pub link: Link<'fs>
}

pub struct LinkData<'fs> {
    pub link_ref: LinkRef<'fs>,
    pub display_text: &'fs str
}

pub enum Link<'fs> {
    ExpandedMDLink(LinkData<'fs>),
    MDLink(LinkData<'fs>),
    ExpandedWikiLink(LinkData<'fs>),
    WikiLink(LinkData<'fs>)
}

pub enum LinkRef<'fs> {
    Heading(&'fs str),
    File(&'fs str),
    Block(&'fs str),
    FileHeading(&'fs str, &'fs str),
    FileBlock(&'fs str, &'fs str)
}

