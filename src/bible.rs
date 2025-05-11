// Bible resource

use std::path::PathBuf;

#[derive(Debug)]
pub struct Bible {
    pub name: String,
    file_path: PathBuf,
}

impl Bible {
    pub fn load(name: &str, file_path: PathBuf) -> Bible {
        Bible {
            name: name.to_string(),
            file_path,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_nothing() {
        let _bible = Bible {
            name: "KJV".into(),
            file_path: PathBuf::from("."),
        };
    }
}
