use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;

pub trait NowProvider {
    fn now(&self) -> NaiveDateTime;
}

pub trait CreateFileProvider<File, Err> {
    fn create_file(&self, path: &Path) -> Result<File, Err>;
}

pub fn find_unique_file_path<T: NowProvider>(
    folder: &PathBuf,
    format: &str,
    now_provider: &T,
) -> PathBuf {
    let now = now_provider.now();

    let filename = now.format(format).to_string();
    let extension = "md";

    find_unique_file_path_rec(folder.into(), filename, extension, None)
}

fn find_unique_file_path_rec(
    path: PathBuf,
    filename: String,
    extension: &str,
    postfix: Option<i32>,
) -> PathBuf {
    let curr_filename = format!(
        "{}{}",
        filename,
        postfix.map_or("".to_string(), |n| n.to_string())
    );

    let filepath = path.join(&curr_filename).with_extension(extension);

    if filepath.exists() {
        find_unique_file_path_rec(
            path,
            filename,
            extension,
            postfix.map_or(Some(0), |n| Some(n + 1)),
        )
    } else {
        filepath
    }
}

pub fn create_unique_note<
    File,
    Err: std::fmt::Debug,
    T: NowProvider + CreateFileProvider<File, Err>,
>(
    folder: &PathBuf,
    format: &str,
    ctx: &T,
) -> PathBuf {
    let file_path = find_unique_file_path(folder, format, ctx);

    // file creation can fail and return an Err, ignore this and try
    // to open the file on the off chance the client knows what to do
    // TODO: log failure to create file
    let _ = file_path.parent().map(std::fs::create_dir_all).unwrap();

    let _ = ctx.create_file(file_path.as_path()).unwrap();

    file_path
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDateTime;

    use crate::unique_notes::{CreateFileProvider, NowProvider};

    use super::create_unique_note;

    struct TestCtx {
        time: NaiveDateTime,
    }

    impl NowProvider for TestCtx {
        fn now(&self) -> NaiveDateTime {
            self.time
        }
    }

    impl CreateFileProvider<(), ()> for TestCtx {
        fn create_file(&self, _path: &std::path::Path) -> Result<(), ()> {
            Ok(())
        }
    }

    #[test]
    fn test_create_unique_note() {
        let folder = PathBuf::from(r"TestFiles/");
        let format = "%Y%m%d%H%M%S";
        let test_ctx = TestCtx {
            time: "2025-05-05T13:01:57".parse::<NaiveDateTime>().unwrap(),
        };

        let expected = PathBuf::from("TestFiles/20250505130157.md");

        let actual = create_unique_note(&folder, format, &test_ctx);

        assert_eq!(actual, expected);
    }
}
