mod named_entity_completion;

use rayon::prelude::*;

use crate::{completions::{Completer, Completion}, context::{Context, Query}};


pub(crate) struct Referencer;

impl<'a> Completer for Referencer {
    fn completions(&self, cx: &Context, location: &crate::completions::Location) -> Option<Vec<Box<dyn crate::completions::Completion>>> {

        let completion_info = cx.parser().link_completion_info(location)?;
        let query = completion_info.query();

        match &query.query {
            Query::Named(named) => {

                let items = cx.querier().named_grep_query(named);
                let items = items
                    .map(|named_entity| Box::new(named_entity) as Box<dyn Completion>)
                    .take(cx.settings().max_query_completion_items())
                    .collect::<Vec<_>>();


                Some(items)

                // map to a Completion

            },
            Query::Block(..) => None
            // Query::Block(block) => {
            //
            //     self.querier.unnamed_grep_query(&block)
            //         .into_par_iter()
            //         .map(|block| Box::new(block) as Box<dyn Completion>)
            //
            // }
        }
    }
}



