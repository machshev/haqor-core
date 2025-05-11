// Bible resource

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

#[derive(Debug)]
pub struct BibleMeta {
    pub description: String,
    pub version: String,
}

#[derive(Debug)]
pub struct Bible {
    pub name: String,
    meta: BibleMeta,
    db: Connection,
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
