//! Source-controlled additions to the people and places inventory and its
//! word-level identifications. The key names a bearer; the integer preserves
//! compatibility with existing runtime foreign keys.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

pub const PATH: &str = "Haqor-Names/review.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bearer {
    pub name: String,
    pub kind: String,
    pub description: String,
}
#[derive(Serialize, Deserialize)]
pub struct Record {
    pub id: u32,
    #[serde(flatten)]
    pub bearer: Bearer,
    pub form: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Mention {
    pub position: u32,
    pub surface: String,
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedNames {
    pub records: BTreeMap<String, Record>,
    /// Edition -> packed scripture reference -> every reviewed position.
    pub verses: BTreeMap<String, BTreeMap<i64, Vec<Mention>>>,
}
impl ReviewedNames {
    pub fn read(src: &Path) -> Result<Self> {
        let path = src.join(PATH);
        match fs::read_to_string(&path) {
            Ok(raw) => {
                serde_json::from_str(&raw).with_context(|| format!("reading {}", path.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn add_record(
        &mut self,
        key: &str,
        bearer: &Bearer,
        form: &str,
        next: &mut u32,
    ) -> Result<()> {
        if let Some(prior) = self.records.get(key) {
            ensure!(
                prior.bearer == *bearer,
                "conflicting descriptions for {key}; reconcile the identity"
            );
        } else {
            *next = next.checked_add(1).context("entity id overflow")?;
            self.records.insert(
                key.to_owned(),
                Record {
                    id: *next,
                    bearer: bearer.clone(),
                    form: form.to_owned(),
                },
            );
        }
        Ok(())
    }
}
