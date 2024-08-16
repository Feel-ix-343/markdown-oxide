use std::{fmt::Formatter, path::Path, sync::Arc};

use anyhow::anyhow;
use itertools::Itertools;
use parsing::{document::{DocSection, Document, Heading, Node}, Documents};
use rayon::prelude::*;

use crate::slot::Slot;


#[derive(Debug)]
pub struct Topics(pub Vec<Arc<Topic>>);

pub type TopicSlot = Slot<Arc<Topic>>;

pub struct Topic {
    parent: Option<TopicSlot>,
    children: Option<Vec<TopicSlot>>,
    title: String
}


impl Topics {
    pub(crate) fn from_documents(documents: &Documents) -> anyhow::Result<Self> {
        let result: anyhow::Result<Vec<_>> = documents.par_iter()
            .map(|(path, doc)| Self::partial_from_document(path, doc))
            .collect();

        Ok(Self(result?.into_par_iter().flatten().collect()))

    }

    fn partial_from_document(path: &Path, document: &Document) -> anyhow::Result<Vec<Arc<Topic>>> {


        let parent_slot = TopicSlot::empty(); // topic for file

        let r: anyhow::Result<Vec<_>> = document.sections.iter()
            .map(|section| match &section.heading {
                Some(heading) => Self::recurse(heading, section, parent_slot.clone()).map(|it| Some(it)),
                None => Ok(None)
            })
            .collect();
        let r: Option<Vec<_>> = r?.into_iter().collect(); // sequence the result

        let (children, children_acc) = match r
            .map(|it| it.into_iter()
                .fold((Vec::new(), Vec::new()), |(mut children, mut children_acc), (child, child_acc)| {
                    children.push(child);
                    children_acc.extend(child_acc);
                    (children, children_acc)
                })) {
            Some((children, children_acc)) => (Some(children), children_acc),
            None => (None, Vec::new())
        };

        let file_name = path.file_stem().ok_or(anyhow!("Path has no file stem"))?.to_str().ok_or(anyhow!("Cannot convert to string"))?.to_string();
        let topic = Topic::new(None, children, file_name);
        parent_slot.set(topic.clone())?;

        let topics = {
            let mut temp = vec![topic];
            temp.extend(children_acc);
            temp
        };

        Ok(topics)


    }

    fn recurse(heading: &Heading, section: &DocSection, parent: TopicSlot) -> anyhow::Result<(TopicSlot, Vec<Arc<Topic>>)> {
        let children_data: Option<Vec<_>> = section.nodes.iter()
            .map(|node| match node {
                Node::Section(section) => match section {
                    DocSection { heading: Some(heading), .. } => Some((heading, section)),
                    _ => None
                }
                _ => None
            }).collect();



        match children_data {
            None => {
                let topic = Topic::new(Some(parent), None, heading.text.to_string());
                let slot = TopicSlot::new(topic.clone());

                Ok((slot, vec![topic]))
            }
            Some(children) => {
                let this_uninitialized = TopicSlot::empty();

                let (children, acc) = children.into_iter()
                    .map(|(child_heading, section)| {
                        Self::recurse(child_heading, section, this_uninitialized.clone())
                    })
                    .fold_ok((Vec::new(), Vec::new()),   |(mut children, mut acc), (child, child_acc)| {
                        children.push(child);
                        acc.extend(child_acc);

                        (children, acc)
                    })?;

                let topic = Topic::new(Some(parent), Some(children), heading.text.to_string());
                let this_slot = this_uninitialized.set(topic.clone())?;


                let mut temp_acc = vec![topic];
                let acc = { 
                    temp_acc.extend(acc);
                    temp_acc
                };

                Ok((this_slot, acc))
            }

        }
    }
}

impl Topic {
    fn new(parent: Option<TopicSlot>, children: Option<Vec<TopicSlot>>, title: String) -> Arc<Self> {
        Arc::new(Topic {
            parent,
            children,
            title
        })
    }
}


// Methods
impl Topics {
    pub(crate) fn is_initialized(&self) -> bool {
        self.0.iter().all(|topic| {
            topic.is_initialized()
        })
    }
}

// Methods
impl Topic {
    fn is_initialized(&self) -> bool {
        if let Some(children) = &self.children {
            if !children.iter().all(|child| {
                child.is_initialized()
            }) {
                return false;
            }
        };

        if let Some(parent) = &self.parent {
            if !parent.is_initialized() {
                return false;
            }
        };

        true
    }
}

use std::fmt::Debug;
impl Debug for Topic {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Topic")
            .field("parent", &self.parent)
            .field("children", &self.children)
            .field("title", &self.title)
            .finish()
    }
}

impl crate::slot::SlotDebug for Arc<Topic> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Topic")
            // .field("parent", &self.parent)
            // .field("children", &self.children)
            .field("title", &self.title)
            .finish()

    }
}

pub mod topic_map {
    use std::{collections::HashMap, path::Path, sync::Arc};

    use derive_deref::Deref;
    use parsing::Documents;
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    use super::TopicSlot;


    #[derive(Debug, Deref)]
    pub struct TopicIDMap(HashMap<TopicID, TopicSlot>);

    #[derive(Debug)]
    pub enum TopicID {
        Path(PathId),
        Unresolved(UnresolvedID)
    }

    #[derive(Debug)]
    pub struct PathId(Arc<Path>, Option<InFileRef>);
    #[derive(Debug)]
    pub struct UnresolvedID(String, Option<InFileRef>);

    // the full path. If there is a sub heading h2 in heading h1, this would be (vec![h1], h2)
    type InFileRef = (Vec<String>, String );

    impl TopicIDMap {
        fn from_documents(documents: &Documents) -> Self {
            documents.par_iter()
                .flat_map(|(path, document)| 
                    document
                        .all_sections()
                        .flat_map(|section| Some((section, &section.heading?)))
                )
        }
    }


}
