use anyhow::Result;
use serde::{Deserialize, Serialize};
use trove::DocumentId;

use crate::bincode;
use crate::relation::Relation;
use crate::text::Text;
use crate::xxhash_rust::xxh3::xxh3_128;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Content {
    Text(Text),
    Relation(Relation),
}

impl Content {
    pub fn id(&self) -> DocumentId {
        let source = match self {
            Content::Text(text) => text.composed_raw().bytes().collect(),
            Content::Relation(relation) => {
                bincode::encode_to_vec(relation, bincode::config::standard()).unwrap()
            }
        };
        DocumentId {
            value: xxh3_128(&source).to_be_bytes(),
        }
    }

    pub fn validated(&self) -> Result<&Self> {
        match self {
            Content::Text(text) => {
                text.validated()?;
            }
            Content::Relation(relation) => {
                relation.validated()?;
            }
        }
        Ok(self)
    }
}
