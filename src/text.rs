use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use trove::DocumentId;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawText(pub String);

impl RawText {
    pub fn validated(&self) -> Result<&Self> {
        static RAW_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let sentence_regex = RAW_REGEX.get_or_init(|| {
            Regex::new(r#"^[0-9\p{Script=Cyrillic}\p{Script=Latin}\s,\-\:\."']+$"#)
                .with_context(|| "Can not compile regular expression for text validation")
                .unwrap()
        });
        if sentence_regex.is_match(&self.0) {
            Ok(self)
        } else {
            Err(anyhow!(
                "Text part around references must be Cyrillic/Latin text: letters, whitespaces, \
                 punctuation ,-:.'\", so {:?} does not seem to be text part",
                self.0
            ))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Text {
    #[serde(default)]
    pub raw_text_parts: Vec<RawText>,
    #[serde(default)]
    pub references: Vec<DocumentId>,
    pub start_with_reference: bool,
}

impl<'a> Text {
    pub fn composed<F>(&self, format_reference: F) -> Result<String>
    where
        F: Fn(&DocumentId) -> Result<String>,
    {
        let mut result_list = Vec::new();
        if self.start_with_reference {
            for (reference_index, reference) in self.references.iter().enumerate() {
                result_list.push(format!("[{}]", format_reference(reference)?));
                if reference_index < self.raw_text_parts.len() {
                    result_list.push(self.raw_text_parts[reference_index].0.clone());
                }
            }
        } else {
            for (part_index, part) in self.raw_text_parts.iter().enumerate() {
                result_list.push(part.0.clone());
                if part_index < self.references.len() {
                    result_list.push(format!(
                        "[{}]",
                        format_reference(&self.references[part_index])?
                    ));
                }
            }
        }
        Ok(result_list.concat())
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
        for part in self.raw_text_parts.iter() {
            part.validated()?;
        }
        Ok(self)
    }
}
