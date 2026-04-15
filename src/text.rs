use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use trove::DocumentId;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Text {
    pub entities: Vec<Entity>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Entity {
    Word(String),
    Reference(DocumentId),
    Other(String),
}

impl<'a> Text {
    pub fn composed<F>(&self, format_reference: F) -> Result<String>
    where
        F: Fn(&DocumentId) -> Result<String>,
    {
        let mut result = String::new();
        for entity in self.entities.iter() {
            match entity {
                Entity::Word(word) | Entity::Other(word) => {
                    result.push_str(&word);
                }
                Entity::Reference(thesis_id) => {
                    result.push_str(&format!("[{}]", &format_reference(&thesis_id)?));
                }
            }
        }
        Ok(result)
    }

    pub fn composed_raw(&self) -> String {
        self.composed(|referenced_thesis_id| {
            Ok(serde_json::to_value(referenced_thesis_id)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string())
        })
        .unwrap()
    }

    pub fn validated(&self) -> Result<&Self> {
        for entity in self.entities.iter() {
            match entity {
                Entity::Word(word) => {
                    static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
                    let sentence_regex = REGEX.get_or_init(|| {
                        Regex::new(r#"^[\p{Script=Cyrillic}\p{Script=Latin}]+$"#)
                            .with_context(|| {
                                "Can not compile regular expression for word validation"
                            })
                            .unwrap()
                    });
                    if !sentence_regex.is_match(&word) {
                        return Err(anyhow!(
                            "Word should be one or more Cyrillic/Latin letters, so {word:?} does \
                             not seem to be a word"
                        ));
                    }
                }
                Entity::Reference(_) => {}
                Entity::Other(_) => {}
            }
        }
        Ok(self)
    }
}
