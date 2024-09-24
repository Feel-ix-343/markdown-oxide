#[derive(Debug)]
pub enum ErrorSet {
    Single(anyhow::Error),
    Multiple(Vec<anyhow::Error>),
}
