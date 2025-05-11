//! # Repository
//!
//! `repo` gives access to bible resources that can be download for use with
//! Haqor. Once resources are downloaded they are stored in a
//! `crate::library::Library`, however the `repo` module is deliberately
//! decoupled from the `crate::library` module so that it can be used
//! independently of it.
//!
//! ## External repos and supported resource formats
//!
//! There are many different bible formats available at the moment. I'm
//! not sure which way to go with this right now in terms of what format Haqor
//! should use. Try and find one that matches our needs, or create a new one.
//!
//! Either way, at the moment the idea is that `repo` provides utilities for
//! interacting with external Repositories... and perhaps later a dedicated
//! Haqor repository. These utilities are agnostic of the actual format and
//! treat the resource as a binary blob at this stage. Later it might provide
//! a common interface for repo agnostic meta data.
//!
//! ## Current status
//!
//! For this MVP we will focus on mysword modules as these are provided in a
//! simple to use SQLite format that can be accessed with standard SQLite
//! library. This also means it's easy to implement lookups and searches as SQL
//! queries.

use anyhow::Result;
use bytes::Bytes;
use log::info;
use reqwest::StatusCode;

#[derive(Debug)]
pub struct ResourceRepo {
    pub name: String,
    url: String,
}

impl Default for ResourceRepo {
    fn default() -> Self {
        ResourceRepo {
            name: "mysword".to_string(),
            url: "https://mysword-bible.info/download/".to_string(),
        }
    }
}

impl ResourceRepo {
    pub fn fetch_bible(&self, name: &str) -> Result<Bytes, String> {
        let bible_url = format!("{}{}.bbl.mybible.gz", self.url, name);

        info!("Downloading {}", bible_url);

        let resp = reqwest::blocking::get(bible_url).expect("request made");

        match resp.status() {
            StatusCode::OK => Ok(resp.bytes().unwrap()),
            s => Err(format!("Error: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_resource_repo() {
        let repo = ResourceRepo::default();

        assert_eq!(repo.name, "mysword");
        assert_eq!(repo.url, "https://mysword-bible.info/download/");
    }

    #[test]
    fn test_resource_repo_fetch_bible() {
        let repo = ResourceRepo::default();

        repo.fetch_bible("kjv");
    }
}
