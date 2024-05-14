use std::fs::File;
use std::path::Path;

use crate::config::Settings;
use chrono::offset::Local;
use chrono::NaiveDateTime;
use fuzzydate::parse;
use serde_json::Value;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::{MessageType, ShowDocumentParams, Url};

fn file_to_datetime(filename: &str, format: &str) -> Option<NaiveDateTime> {
    // re-hydrate a datetime from a dailynote filename
    todo!()
}

fn datetime_to_file(datetime: NaiveDateTime, settings: &Settings) -> Option<Url> {
    let filename = datetime.format(&settings.dailynote).to_string();
    let path = Path::new(&filename);

    Url::from_file_path(path.with_extension("md")).ok()
}

pub async fn jump(
    client: &tower_lsp::Client,
    settings: &Settings,
    jump_to: Option<&str>,
) -> Result<Option<Value>> {
    // if jump_to is None, use the current time.

    let note_file = match jump_to {
        Some(jmp_str) => parse(jmp_str)
            .ok()
            .and_then(|dt| datetime_to_file(dt, &settings)),
        None => datetime_to_file(Local::now().naive_local(), &settings),
    };

    if let Some(uri) = note_file {
        // file creation can fail and return an Err, ignore this and try
        // to open the file on the off chance the client knows what to do
        // TODO: log failure to create file
        let _ = uri.to_file_path().map(|path| {
            path.parent().map(|parent| std::fs::create_dir_all(parent));
            File::create_new(path.as_path().to_owned())
        });

        client
            .show_document(ShowDocumentParams {
                uri,
                external: Some(false),
                take_focus: Some(true),
                selection: None,
            })
            .await
            .map(|success| Some(success.into()))
    } else {
        client
            .log_message(
                MessageType::ERROR,
                format!("could not parse {jump_to:?}: {:?}", jump_to.map(parse)),
            )
            .await;
        Err(Error::invalid_params(
            "Could not parse journal format as a valid uri.".to_string(),
        ))
    }
}

pub fn jump_relative(
    client: tower_lsp::Client,
    settings: &Settings,
    jump_to: &str,
) -> Result<Option<Value>> {
    todo!("pending PR in fuzzydate to specify base time")
}
