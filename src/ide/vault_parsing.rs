use std::{path::Path, collections::HashMap};

use itertools::Itertools;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use super::nodes::{MDFile, MDHeading};

pub fn parse_vault(dir: &str) -> Result<(), std::io::Error>  {

    let dir_path = Path::new(dir).to_owned();



    let md_files: HashMap<String, MDFile> = dir_path
        .read_dir()?
        .filter_map(|f| Result::ok(f))
        .collect_vec()
        .par_iter()
        .filter(|f| f.path().extension().and_then(|e| e.to_str()) == Some("md"))
        .map(|f| {

            let md_file = MDFile::new(f.path());

            let relative_path = f.path().strip_prefix(&dir_path).unwrap().to_owned();
            let ref_name = relative_path.file_stem().unwrap().to_owned(); // TODO: Make sure that this did not mess up folders

            return (ref_name.to_str().unwrap().to_owned(), md_file)
        })
        .collect();

    // Map of all headings by obsidian style refname
    let headings: HashMap<String, &MDHeading> = md_files.iter()
        .flat_map(|(s, f)| {
            f.headings.iter().map(move |h| {
                let ref_name = format!("{}#{}", s, h.heading_text);
                    (ref_name, h)
            })
        })
        .collect();


    // TODO: Tags, lists, ... more specific thigns
    return Ok(())
}
