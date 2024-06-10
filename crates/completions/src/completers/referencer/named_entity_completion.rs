use crate::{completions::Completion, context::{Context, EntityWithBacklinksPreview, FileEntityType, LinkingStyle, NamedEntity, NamedEntityInfo, UnresolvedFileEntityType}};


/// UI implementation for named entities

impl Completion for NamedEntity {
    fn label(&self, cx: &Context) -> String {

        match self.info() {
            NamedEntityInfo::File { path, entity_type } => {

                let file_ref = path.file_name().unwrap().to_str().expect("file name to be a valid string");

                match entity_type {
                    FileEntityType::Normal => {
                        if cx.settings().file_as_first_heading() {
                            cx.querier().first_heading_of_file(&path).unwrap_or(file_ref).to_string()
                        } else {
                            file_ref.to_string()
                        }
                    }
                    FileEntityType::DailyNote { relative_label } => {
                        relative_label.to_string()
                    }
                    FileEntityType::Alias { alias } => {
                        alias.to_string()
                    }
                }


            }
            NamedEntityInfo::Heading { heading, file } => {

                let filename = file.file_name().unwrap().to_str().expect("file name to be a valid string");

                // TODO: implement the matched text for long headings
                format!("{filename}#{heading}")

            }
            NamedEntityInfo::Block { index, file } => {

                let file_name = file.file_name().unwrap().to_str().expect("file name to be a valid string");
                format!("{file_name}#{index}")

            }

            NamedEntityInfo::UnresovledFile { file_ref, entity_type } => {
                match entity_type {
                    UnresolvedFileEntityType::Normal => {
                        file_ref.to_string()
                    }
                    UnresolvedFileEntityType::DailyNote { relative_label } => {
                        relative_label.to_string()
                    }
                }
            }

            NamedEntityInfo::UnresolvedHeading { file_ref, heading } => {
                format!("{file_ref}#{heading}")
            }
        }

        // TODO: relative path and absolute path

    }

    fn label_detail(&self, cx: &Context) -> Option<String> {
        match self.info() {
            NamedEntityInfo::UnresolvedHeading {..} | NamedEntityInfo::UnresovledFile {..} => Some("Unresolved".to_string()),
            NamedEntityInfo::File {path, entity_type:  FileEntityType::Alias {..}  } => {
                let file_name = path.file_name().unwrap().to_str().expect("file name to be a valid string");

                Some(format!("Alias for {file_name}"))
            }
            _ => None
        }
    }

    fn kind(&self, cx: &Context) -> tower_lsp::lsp_types::CompletionItemKind {
        match self.info() {
            NamedEntityInfo::File { entity_type, .. } => {
                match entity_type {
                    FileEntityType::Normal => tower_lsp::lsp_types::CompletionItemKind::FILE,
                    FileEntityType::DailyNote { .. } => tower_lsp::lsp_types::CompletionItemKind::ENUM_MEMBER,
                    FileEntityType::Alias { .. } => tower_lsp::lsp_types::CompletionItemKind::ENUM_MEMBER
                }
            }
            NamedEntityInfo::Heading { .. } => tower_lsp::lsp_types::CompletionItemKind::REFERENCE,
            NamedEntityInfo::Block { .. } => tower_lsp::lsp_types::CompletionItemKind::REFERENCE,
            NamedEntityInfo::UnresovledFile { .. } => tower_lsp::lsp_types::CompletionItemKind::ENUM_MEMBER,
            NamedEntityInfo::UnresolvedHeading { .. } => tower_lsp::lsp_types::CompletionItemKind::ENUM_MEMBER
        }
    }

    fn detail(&self, cx: &Context) -> Option<String> {
        None
    }

    fn documentation(&self, cx: &Context) -> Option<String> {
        let EntityWithBacklinksPreview { entity_preview, backlinks } = cx.entity_view().preview_with_backlinks(self);

        let backlinks_string = backlinks.take(cx.settings().backlinks_to_preview())
            .map(|(_, text)| text)
            .collect::<Vec<_>>().join("\n");

        let backlinks_string = if backlinks_string.is_empty() {
            "".to_string()
        } else {
            format!("\n---\n{}", backlinks_string)
        };

        Some(format!("{entity_preview}{backlinks_string}"))
    }

    fn deprecated(&self, cx: &Context) -> Option<bool> {
        None
    }

    fn preselect(&self, cx: &Context) -> Option<bool> {

        let entered_query = cx.parser().entered_query_string();

        Some(match self.info() {
            NamedEntityInfo::File { path, entity_type: FileEntityType::DailyNote { relative_label } } => {
                entered_query == relative_label
            }
            NamedEntityInfo::File { path, entity_type: FileEntityType::Normal  } if cx.settings().file_as_first_heading() => {
                let first_heading = cx.querier().first_heading_of_file(&path)?;

                entered_query == first_heading
            }
            NamedEntityInfo::File { path, entity_type: FileEntityType::Normal  } => {

                let name = path.file_name().unwrap().to_str().expect("file name to be a valid string");

                entered_query == name
            }
            NamedEntityInfo::File { path, entity_type: FileEntityType::Alias { alias }  } => {
                entered_query == alias
            }
            NamedEntityInfo::UnresovledFile { file_ref, entity_type: UnresolvedFileEntityType::Normal } => {
                file_ref == entered_query
            }
            NamedEntityInfo::UnresovledFile { file_ref, entity_type: UnresolvedFileEntityType::DailyNote { relative_label } } => {
                relative_label == entered_query
            }
            NamedEntityInfo::Heading { heading, file } => false,
            NamedEntityInfo::Block { index, file } => false,
            NamedEntityInfo::UnresolvedHeading { file_ref: file_Ref, heading } => false,
        })

    }

    fn text_edit(&self, cx: &Context) -> tower_lsp::lsp_types::TextEdit {
        let (display_text, file_ref, heading_ref): (Option<String>, String, Option<String>) = match self.info() {

            NamedEntityInfo::File { path, entity_type: FileEntityType::Normal } if cx.settings().file_as_first_heading() => {
                (cx.querier().first_heading_of_file(&path).map(ToString::to_string), path.file_name().unwrap().to_str().expect("file name to be a valid string").to_string(), None)
            }
            NamedEntityInfo::File { path, entity_type: FileEntityType::Normal } => {
                (None, path.file_name().unwrap().to_str().expect("file name to be a valid string").to_string(), None)
            }
            NamedEntityInfo::File { path, entity_type: FileEntityType::Alias { alias } } => {
                (Some(alias.to_string()), path.file_name().unwrap().to_str().expect("file name to be a valid string").to_string(), None)
            }
            NamedEntityInfo::File { path, entity_type: FileEntityType::DailyNote { relative_label } } => {
                (Some(relative_label.to_string()), path.file_name().unwrap().to_str().expect("file name to be a valid string").to_string(), None)
            }
            NamedEntityInfo::UnresovledFile { file_ref, entity_type } => {
                (None, file_ref.to_string(), None)
            }
            NamedEntityInfo::Heading { heading, file } => {
                (Some(heading.to_string()), file.file_name().unwrap().to_str().expect("file name to be a valid string").to_string(), Some(heading.to_string()))
            }
            NamedEntityInfo::Block { index, file } => {
                (None, file.file_name().unwrap().to_str().expect("file name to be a valid string").to_string(), Some(format!("^{}", index)))
            },
            NamedEntityInfo::UnresolvedHeading { file_ref, heading } => {
                (Some(heading.to_string()), file_ref.to_string(), Some(heading.to_string()))
            },

        };

        let new_text = match cx.parser().linking_style() {
            LinkingStyle::Markdown => {
                let ref_text = |file: String, heading| {
                    if file.contains(" ") || heading_ref.clone().is_some_and(|it| it.contains(" ")) {
                        format!("<{}{}>", file, heading)
                    } else {
                        format!("{}{}", file, heading)
                    }
                };

                let heading = heading_ref.as_ref().map(|it| format!("#{}", it)).unwrap_or("".to_string());
                let display_text = display_text.unwrap_or("".to_string());

                format!("[{}]({})", display_text, ref_text(file_ref, heading))
            }
            LinkingStyle::Wikilink => {
                let display = |d| format!("|{}", d);
                let heading = heading_ref.map(|it| format!("#{}", it)).unwrap_or("".to_string());
                let display_text = display_text.map(display).unwrap_or("".to_string());

                format!("[[{file_ref}{heading}{display_text}]]")
            }

        };

        let range = cx.parser().link_range();

        tower_lsp::lsp_types::TextEdit {
            range,
            new_text
        }
    }

    fn command(&self, cx: &Context) -> Option<tower_lsp::lsp_types::Command> {
        None
    }

    
}

// NOTE: This is here instead of in the struct functions becuaes it is this module that is responsible for what the *display
// text is, not the named entity struct. This is a UI responsibility. 
fn display_text(named: &NamedEntity) -> String {
    todo!()
}
