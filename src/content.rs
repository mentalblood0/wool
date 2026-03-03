use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use trove::DocumentId;

use crate::relation::Relation;
use crate::text::Text;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Text(Text),
    Relation(Relation),
}

impl Content {
    pub fn id(&self) -> Result<DocumentId> {
        let source = match self {
            Content::Text(text) => text.composed().bytes().collect(),
            Content::Relation(relation) => {
                bincode::encode_to_vec(relation, bincode::config::standard()).with_context(
                    || {
                        format!(
                            "Can not binary encode Content {self:?} in order to compute it's \
                             DocumentId as it's binary representation hash"
                        )
                    },
                )?
            }
        };
        Ok(DocumentId {
            value: xxhash_rust::xxh3::xxh3_128(&source).to_be_bytes(),
        })
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
