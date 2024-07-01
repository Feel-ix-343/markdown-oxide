use crate::{
    command::{LinkBlockCmd, ReferenceNamedSectionCmd},
    context::Context,
    entity::{Block, Entity, NamedEntityTypeInfo::*},
    parser::{BlockLinkCmdQuery, EntityInfileQuery, NamedRefCmdQuery, QueryMetadata},
};
use matcher::{run_query, Query, Queryable};
use rayon::prelude::*;
use vault::Vault;

mod matcher;

pub fn query_named_ref_cmds<'a, 'b: 'a>(
    cx: &'a Context<'a>,
    query_metadata: &QueryMetadata,
    cmd_query: &'b NamedRefCmdQuery, // TODO: I think this is a rust bug; lifetime is not needed
) -> impl IndexedParallelIterator<Item = ReferenceNamedSectionCmd<'a>> {
    let all_cmds = cx.querier().get_named_ref_cmds(cx);
    let binding = all_cmds.collect::<Vec<_>>();
    let iterator = binding.into_iter();
    let matched = run_query(cmd_query, iterator);

    matched
}

pub fn query_block_link_cmds<'a, 'b: 'a>(
    cx: &'a Context<'a>,
    query_metadata: &QueryMetadata,
    cmd_query: &'b BlockLinkCmdQuery,
) -> impl IndexedParallelIterator<Item = LinkBlockCmd<'a>> {
    let all_cmds = cx.querier().get_block_link_cmds(cx);
    let binding = all_cmds.collect::<Vec<_>>();
    let iterator = binding.into_iter();
    let matched = run_query(cmd_query, iterator);

    matched
}

pub struct Querier<'a> {
    vault: &'a Vault,
}

impl<'a> Querier<'a> {
    pub fn new(vault: &'a Vault) -> Self {
        Self { vault }
    }
}

impl<'a> Querier<'a> {
    fn get_named_ref_cmds(
        &self,
        context: &Context,
    ) -> impl IndexedParallelIterator<Item = ReferenceNamedSectionCmd> {
        todo!();
        vec![].into_par_iter()
    }

    fn get_block_link_cmds(&self, context: &Context) -> impl ParallelIterator<Item = LinkBlockCmd> {
        todo!();
        vec![].into_par_iter()
    }
}

impl Query for BlockLinkCmdQuery<'_> {
    fn to_query_string(&self) -> String {
        self.grep_string.to_string()
    }
}

impl Query for NamedRefCmdQuery<'_> {
    fn to_query_string(&self) -> String {
        match self {
            NamedRefCmdQuery {
                file_query: file_ref,
                infile_query: None,
            } => file_ref.to_string(),
            NamedRefCmdQuery {
                file_query: file_ref,
                infile_query: Some(EntityInfileQuery::Heading(heading_string)),
            } => format!("{file_ref}#{heading_string}"),
            NamedRefCmdQuery {
                file_query: file_ref,
                infile_query: Some(EntityInfileQuery::Index(index)),
            } => format!("{file_ref}#^{index}"),
        }
    }
}

impl Queryable for Entity<'_> {
    fn match_string(&self) -> String {
        let file_ref = self.info.path.file_name().unwrap().to_str().unwrap();

        match self.info.type_info {
            Heading(heading) => format!("{file_ref}#{heading}"),
            IndexedBlock(index) => format!("{file_ref}#^{index}"),
            _ => file_ref.to_string(),
        }
    }
}

impl Queryable for Block<'_> {
    fn match_string(&self) -> String {
        self.info.line_text.to_string()
    }
}

impl Queryable for ReferenceNamedSectionCmd<'_> {
    fn match_string(&self) -> String {
        todo!()
    }
}

impl Queryable for LinkBlockCmd<'_> {
    fn match_string(&self) -> String {
        todo!()
    }
}
