use std::{ops::Range, path::Path};

use super::FileRange;


pub struct MDFile<'fs> {
    metadata: MDMetadata<'fs>,
    base_section: MDSection<'fs>,
    footnotes: Vec<MDFootnoteDefinition<'fs>>,
    link_ref_defs: Vec<MDLinkReferenceDefinition<'fs>>
}

pub struct Ranged<'fs, T> {
    range: FileRange<'fs>,
    item: T
}


pub struct MDSection<'fs> {
    starter: Ranged<'fs, MDSectionStarter<'fs>>,
    subsections: Vec<Ranged<'fs, MDSubsection<'fs>>>,
    children: Vec<MDSection<'fs>>
}

pub enum MDSectionStarter<'fs> {
    File(&'fs Path),
    Heading(MDHeading<'fs>),
    Separator,
    Footnote,
    LinkRefDef,
}


pub enum MDSubsection<'fs> {
    Paragraph(Ranged<'fs, MDBlock<'fs>>),
    List(Ranged<'fs, MDList<'fs>>),
    CodeBlock(Ranged<'fs, CodeBlockFull<'fs>>),
    BlockQuote(Ranged<'fs, Blockquote<'fs>>),
    Callout(Ranged<'fs, MDCallout<'fs>>)
}

pub enum MDList<'fs> {
    Ordered(Vec<(Ranged<'fs, MDOrderedListItem<'fs>>, MDList<'fs>)>),
    Unordered(Vec<(Ranged<'fs, MDUnorderedListItem<'fs>>, MDList<'fs>)>)
}

pub struct Spanned<'fs, T> {
    range: Range<u8>
}


pub struct MDBlock<'fs> {
    text: &'fs str,
    tags: Vec<Spanned<'fs, MDTag<'fs>>>,
    links: Vec<Spanned<'fs, Link<'fs>>>,
    short_codeblocks: Vec<Spanned<'fs, CodeBlockShort<'fs>>>,
    index: Option<Spanned<'fs, &'fs str>>
}




pub struct MDMetadata<'fs> {
}

pub struct MDHeading<'fs> {
    heading_text: &'fs str,
    heading_level: u8,
}


pub struct MDTag<'fs> {
}


pub struct WikiLinkMDFile<'fs> {
}

pub struct WikiLinkHeading<'fs> {
}

pub struct WikilinkBlock<'fs> {
}

pub struct WikilinkOther<'fs> {
}

pub enum WikiLink<'fs> {
    MDFile(WikiLinkMDFile<'fs>),
    MDHeading(WikiLinkHeading<'fs>),
    MDBlock(WikilinkBlock<'fs>),
    MDOther(WikilinkOther<'fs>),
}

pub struct MarkdownLinkMDFile<'fs> {
}

pub struct MarkdownLinkHeading<'fs> {
}

pub struct MarkdownLinkBlock<'fs> {
}

pub struct MarkdownLinkOther<'fs> {

}

pub enum MarkdownLink<'fs> {
    MDFile(MarkdownLinkMDFile<'fs>),
    MDHeading(MarkdownLinkHeading<'fs>),
    MDBlock(MarkdownLinkBlock<'fs>),
    MDOther(MarkdownLinkOther<'fs>),
}

pub enum MDLink<'fs> {
    WikiLink(WikiLink<'fs>),
    MarkdownLink(MarkdownLink<'fs>),
    Footnote(MDFootnoteLink<'fs>),
    DefRef(MDLinkReference<'fs>)
}

pub enum Link<'fs> {
    Expanded(MDLink<'fs>),
    Unexpanded(MDLink<'fs>),
}


pub struct CodeBlockFull<'fs> {

}

pub struct CodeBlockShort<'fs> {
}

pub struct MDFootnoteLink<'fs> {

}

pub struct MDFootnoteDefinition<'fs> {

}

pub struct MDLinkReference<'fs> {
}

pub struct MDLinkReferenceDefinition<'fs> {

}

pub struct Blockquote<'fs> {

}

pub struct MDCallout<'fs> {

}

pub enum CalloutType {
    Note,
    Abstract,
    Summary,
    Tldr,
    Info,
    Todo,
    Tip,
    Hint,
    Important,
    Success,
    Check,
    Done,
    Question,
    Help,
    Faq,
    Warning,
    Caution,
    Attention,
    Failure,
    Fail,
    Missing,
    Danger,
    Error,
    Bug,
    Example,
    Quote,
    Cite,
}

pub enum MDPartialListItem<'fs> {
    Basic(MDBlock<'fs>),
    Task(MDBlock<'fs>),
}

pub struct MDOrderedListItem<'fs> {
    partial_list_item: MDPartialListItem<'fs>
}

pub struct MDUnorderedListItem<'fs> {
    partial_list_item: MDPartialListItem<'fs>
}

pub enum MDListItem<'fs> {
    Ordered(MDOrderedListItem<'fs>),
    Unordered(MDUnorderedListItem<'fs>),
}
