use anyhow::Result;
use serde::{Deserialize, Serialize};
use trove::DocumentId;

use crate::bincode;
use crate::relation_kind::RelationKind;

#[derive(Serialize, Deserialize, Debug, Clone, bincode::Encode, PartialEq, Eq)]
#[bincode(crate = "bincode")]
pub struct Relation {
    pub from: DocumentId,
    pub to: DocumentId,
    pub kind: RelationKind,
}

impl Relation {
    pub fn validated(&self) -> Result<&Self> {
        self.kind.validated()?;
        Ok(self)
    }
}
