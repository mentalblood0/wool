use anyhow::Result;
use serde::{Deserialize, Serialize};
use trove::DocumentId;

use crate::alias::Alias;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reference {
    Alias(Alias),
    DocumentId(DocumentId),
}

impl Reference {
    pub fn new(input: &str) -> Result<Self> {
        if let Ok(document_id) =
            serde_json::from_value::<DocumentId>(serde_json::Value::String(input.to_string()))
        {
            Ok(Self::DocumentId(document_id))
        } else {
            Ok(Self::Alias(
                Alias(input.to_string()).validated()?.to_owned(),
            ))
        }
    }
}
