// Bible resource

use anyhow::Result;
use rusqlite::Connection;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub struct BibleMeta {
    pub description: String,
    pub version: String,
}

impl fmt::Display for BibleMeta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Description: {}\nversion: {}",
            self.description, self.version
        )
    }
}

#[derive(Debug)]
pub struct Bible {
    pub name: String,
    meta: BibleMeta,
    db: Connection,
}

impl fmt::Display for Bible {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name: {}\n{}", self.name, self.meta)
    }
}

impl Bible {
    pub fn load(name: &str, file_path: PathBuf) -> Result<Bible> {
        if !file_path.exists() {
            panic!("Bible file doesn't exist")
        }

        let db = Connection::open(&file_path)?;

        let meta = db.query_row("SELECT Title, Version from Details", (), |row| {
            Ok(BibleMeta {
                description: row.get(0)?,
                version: row.get(1)?,
            })
        })?;

        Ok(Bible {
            name: name.to_string(),
            meta,
            db,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_nothing() {
        let _bible = Bible {
            name: "KJV".into(),
            meta: BibleMeta {
                description: "Description".into(),
                version: "2.9.4".into(),
            },
            db: Connection::open_in_memory().unwrap(),
        };
    }
}
