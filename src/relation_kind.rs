use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::bincode;

#[derive(Serialize, Deserialize, Debug, Clone, bincode::Encode, PartialEq, Eq, PartialOrd, Ord)]
#[bincode(crate = "bincode")]
pub struct RelationKind(pub String);

impl RelationKind {
    pub fn validated(&self) -> Result<&Self> {
        static RELATION_KIND_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let sentence_regex = RELATION_KIND_REGEX.get_or_init(|| {
            Regex::new(r"^[\w ]+$")
                .with_context(|| "Can not compile regular expression for relation kind validation")
                .unwrap()
        });
        if sentence_regex.is_match(&self.0) {
            Ok(self)
        } else {
            Err(anyhow!(
                "Relation kind should be an English words sequence without punctuation, so {:?} \
                 does not seem to be relation kind",
                self.0
            ))
        }
    }
}
