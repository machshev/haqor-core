// Library of resources

use log::info;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::bible::Bible;

#[derive(Debug)]
pub struct Library {
    base_path: PathBuf,
}

impl Default for Library {
    fn default() -> Self {
        let base_path: PathBuf = match env::var("HAQOR_LIBRARY_PATH") {
            Ok(i) => PathBuf::from(i),
            _ => dirs::data_dir()
                .expect("Can't access data dir")
                .join("haqor/library"),
        };

        Library { base_path }
    }
}

impl Library {
    pub fn get_library(base_path: &Path) -> Library {
        Library {
            base_path: base_path.to_path_buf(),
        }
    }

    pub fn save_bible(&self, name: &str, content: &[u8]) {
        let file_path = self.base_path.join(format!("{}.bbl.mybible.gz", name));

        info!("Saving bible to {:?}", file_path);

        std::fs::create_dir_all(self.base_path.as_path())
            .expect("failed to create parent directories");

        let mut out = File::create(file_path).expect("failed to create file");
        out.write_all(content).expect("failed to write file");
    }

    pub fn get_bible(&self, name: &str) -> Bible {
        let file_path: PathBuf = self.base_path.join(format!("{}.bbl.mybible.gz", name));

        log::info!(
            "Loading bible '{}' from '{:?}' exists '{}'",
            name,
            file_path,
            file_path.exists()
        );

        Bible::load(&name, file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_library() {
        let lib = Library::default();

        assert_eq!(
            lib.base_path,
            dirs::data_dir()
                .expect("Can't access data dir")
                .join("haqor/library/")
        );
    }

    #[test]
    fn test_get_library() {
        let lib = Library::get_library(Path::new("../test_library"));

        assert_eq!(lib.base_path, Path::new("../test_library"));
    }

    #[test]
    fn test_get_bible() {
        let lib = Library::default();
        let bible = lib.get_bible("test_bible");

        assert_eq!(bible.name, "test_bible");
    }

    /*
    #[test]
    fn get_bible_description(){
        let result = get_bible_description("test_bible");
    }*/
}
