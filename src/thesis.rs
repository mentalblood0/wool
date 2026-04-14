use anyhow::Result;
use serde::{Deserialize, Serialize};
use trove::DocumentId;

use crate::alias::Alias;
use crate::command::Command;
use crate::content::Content;
use crate::relation::Relation;
use crate::tag::Tag;
use crate::text::Text;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Thesis {
    pub alias: Option<Alias>,
    pub content: Content,

    #[serde(default)]
    pub tags: Vec<Tag>,
}

impl Thesis {
    pub fn id(&self) -> DocumentId {
        self.content.id()
    }

    pub fn validated(&self) -> Result<&Self> {
        if let Some(ref alias) = self.alias {
            alias.validated()?;
        }
        self.content.validated()?;
        for tag in self.tags.iter() {
            tag.validated()?;
        }
        Ok(self)
    }

    pub fn references(&self) -> Vec<DocumentId> {
        match self.content {
            Content::Text(Text {
                raw_text_parts: _,
                ref references,
                start_with_reference: _,
            }) => references.clone(),
            Content::Relation(Relation {
                ref from,
                ref to,
                kind: _,
            }) => vec![from.clone(), to.clone()],
        }
    }

    pub fn to_commands(&self) -> Vec<Command> {
        let mut result = Vec::with_capacity(2);
        let self_without_tags = Thesis {
            content: self.content.clone(),
            alias: self.alias.clone(),
            tags: vec![],
        };
        result.push(match self.content {
            Content::Text(_) => match self.alias {
                Some(_) => Command::AddTextThesisWithAlias(self_without_tags),
                None => Command::AddTextThesisWithoutAlias(self_without_tags),
            },
            Content::Relation(_) => match self.alias {
                Some(_) => Command::AddRelationThesisWithAlias(self_without_tags),
                None => Command::AddRelationThesisWithoutAlias(self_without_tags),
            },
        });
        if !self.tags.is_empty() {
            result.push(Command::AddTags {
                thesis_id: self.id(),
                tags: self.tags.clone(),
            })
        }
        result
    }
}
