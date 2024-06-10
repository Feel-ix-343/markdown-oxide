use std::path::{Path, PathBuf};

use completions::Location;
use tower_lsp::lsp_types::{CompletionParams, CompletionResponse};
use vault::Vault;
use moxide_config::Settings;

use crate::{completers::referencer::Referencer, completions::completions, context::Context};

mod completions;
mod context;
mod completers;


pub fn get_completions(
    vault: &Vault,
    files: &Box<[PathBuf]>,
    params: &CompletionParams,
    path: &Path,
    settings: &Settings
) -> Option<CompletionResponse> {


    // init context
    let cx = Context::new(vault, settings);

    // init completers
    let referencer = Referencer;

    let location = Location {
        line: params.text_document_position.position.line,
        character: params.text_document_position.position.character,
        file: path.to_string_lossy().to_string()
    };

    completions(&location, &cx, &referencer)

}



