use std::collections::BTreeSet;

use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use trove::DocumentId;

use crate::alias::Alias;
use crate::aliases_resolver::AliasesResolver;
use crate::content::Content;
use crate::reference::Reference;
use crate::relation::Relation;
use crate::relation_kind::RelationKind;
use crate::tag::Tag;
use crate::thesis::Thesis;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Command {
    AddTextThesisWithAlias(Thesis),
    AddTextThesisWithoutAlias(Thesis),
    AddRelationThesisWithAlias(Thesis),
    AddRelationThesisWithoutAlias(Thesis),
    SetAlias {
        thesis_id: DocumentId,
        alias: Alias,
    },
    AddTags {
        thesis_id: DocumentId,
        tags: Vec<Tag>,
    },
    RemoveTags {
        thesis_id: DocumentId,
        tags: Vec<Tag>,
    },
}

impl Command {
    pub fn validated(self) -> Result<Self> {
        match self {
            Command::AddTextThesisWithAlias(ref thesis) => {
                thesis.validated()?;
            }
            Command::AddTextThesisWithoutAlias(ref thesis) => {
                thesis.validated()?;
            }
            Command::AddRelationThesisWithAlias(ref thesis) => {
                thesis.validated()?;
            }
            Command::AddRelationThesisWithoutAlias(ref thesis) => {
                thesis.validated()?;
            }
            Command::SetAlias { ref alias, .. } => {
                alias.validated()?;
            }
            Command::AddTags { ref tags, .. } => {
                for tag in tags.iter() {
                    tag.validated()?;
                }
            }
            Command::RemoveTags { ref tags, .. } => {
                for tag in tags.iter() {
                    tag.validated()?;
                }
            }
        }
        Ok(self)
    }

    fn parse_as_set_alias<'a, 'b>(
        line: &str,
        aliases_resolver: &mut dyn AliasesResolver,
    ) -> Result<Self> {
        static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let regex = REGEX.get_or_init(|| {
            Regex::new(r#"^/may +(\S+) +alias +(\S+)$"#)
                .with_context(|| "Can not compile regular expression")
                .unwrap()
        });
        if let Some(captures) = regex.captures(line) {
            let alias_capture = &captures[1];
            let reference_capture = &captures[2];
            let alias = Alias(alias_capture.to_string()).validated()?.to_owned();
            let thesis_id = aliases_resolver
                .get_thesis_id_by_reference(&Reference::new(&reference_capture)?)?;
            let result = Self::SetAlias {
                thesis_id: thesis_id.clone(),
                alias: alias.clone(),
            }
            .validated()?;
            aliases_resolver.remember(alias, thesis_id);
            Ok(result)
        } else {
            Err(anyhow!(
                "Can not match {line:?} with regular expression {REGEX:?}"
            ))
        }
    }

    fn parse_as_add_relation_thesis_with_alias<'a, 'b>(
        line: &str,
        aliases_resolver: &mut dyn AliasesResolver,
        supported_relations_kinds: &BTreeSet<RelationKind>,
    ) -> Result<Self> {
        static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let regex = REGEX.get_or_init(|| {
            Regex::new(r#"^/may +(\S+) alias +(\S+) +(.+) +(\S+)$"#)
                .with_context(|| "Can not compile regular expression")
                .unwrap()
        });
        if let Some(captures) = regex.captures(line) {
            let alias_capture = &captures[1];
            let from_reference_capture = &captures[2];
            let relation_kind_capture = &captures[3];
            let to_reference_capture = &captures[4];
            let relation_kind = RelationKind(relation_kind_capture.to_string());
            if !supported_relations_kinds.contains(&relation_kind) {
                return Err(anyhow!("Relation kind {relation_kind:?} is not supported"));
            }
            let alias = Alias(alias_capture.to_string()).validated()?.to_owned();
            let thesis = Thesis {
                alias: Some(alias.clone()),
                content: Content::Relation(Relation {
                    from: aliases_resolver
                        .get_thesis_id_by_reference(&Reference::new(from_reference_capture)?)?,
                    kind: relation_kind,
                    to: aliases_resolver
                        .get_thesis_id_by_reference(&Reference::new(to_reference_capture)?)?,
                }),
                tags: vec![],
            };
            let id = thesis.id()?;
            let result = Self::AddRelationThesisWithAlias(thesis).validated()?;
            aliases_resolver.remember(alias, id);
            Ok(result)
        } else {
            Err(anyhow!(
                "Can not match {line:?} with regular expression {REGEX:?}"
            ))
        }
    }

    fn parse_as_add_relation_thesis_without_alias<'a, 'b>(
        line: &str,
        aliases_resolver: &mut dyn AliasesResolver,
        supported_relations_kinds: &BTreeSet<RelationKind>,
    ) -> Result<Self> {
        static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let regex = REGEX.get_or_init(|| {
            Regex::new(r#"^/may +(\S+) +(.+) +(\S+)$"#)
                .with_context(|| "Can not compile regular expression")
                .unwrap()
        });
        if let Some(captures) = regex.captures(line) {
            let from_reference_capture = &captures[1];
            let relation_kind_capture = &captures[2];
            let to_reference_capture = &captures[3];
            let relation_kind = RelationKind(relation_kind_capture.to_string());
            if !supported_relations_kinds.contains(&relation_kind) {
                return Err(anyhow!("Relation kind {relation_kind:?} is not supported"));
            }
            let to = aliases_resolver
                .get_thesis_id_by_reference(&Reference::new(to_reference_capture)?)?;
            let result = Self::AddRelationThesisWithoutAlias(Thesis {
                alias: None,
                content: Content::Relation(Relation {
                    from: aliases_resolver
                        .get_thesis_id_by_reference(&Reference::new(from_reference_capture)?)?,
                    kind: relation_kind,
                    to: to.clone(),
                }),
                tags: vec![],
            })
            .validated()?;
            Ok(result)
        } else {
            Err(anyhow!(
                "Can not match {line:?} with regular expression {REGEX:?}"
            ))
        }
    }

    fn parse_as_add_text_thesis_with_alias<'a, 'b>(
        line: &str,
        aliases_resolver: &mut dyn AliasesResolver,
    ) -> Result<Self> {
        static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let regex = REGEX.get_or_init(|| {
            Regex::new(r#"^/may +(\S+) +alias +(.+)$"#)
                .with_context(|| "Can not compile regular expression")
                .unwrap()
        });
        if let Some(captures) = regex.captures(line) {
            let alias_capture = &captures[1];
            let thesis_text_capture = &captures[2];
            let alias = Alias(alias_capture.to_string()).validated()?.to_owned();
            let thesis = Thesis {
                alias: Some(alias.clone()),
                content: Content::Text(aliases_resolver.new_text(&thesis_text_capture)?),
                tags: vec![],
            };
            aliases_resolver.remember(alias, thesis.id()?);
            Self::AddTextThesisWithAlias(thesis).validated()
        } else {
            Err(anyhow!(
                "Can not match {line:?} with regular expression {REGEX:?}"
            ))
        }
    }

    fn parse_as_add_text_thesis_without_alias<'a, 'b>(
        line: &str,
        aliases_resolver: &mut dyn AliasesResolver,
    ) -> Result<Self> {
        static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let regex = REGEX.get_or_init(|| {
            Regex::new(r#"^/may +(.+)$"#)
                .with_context(|| "Can not compile regular expression")
                .unwrap()
        });
        if let Some(captures) = regex.captures(line) {
            let thesis_text_capture = &captures[1];
            Self::AddTextThesisWithoutAlias(Thesis {
                alias: None,
                content: Content::Text(aliases_resolver.new_text(thesis_text_capture)?),
                tags: vec![],
            })
            .validated()
        } else {
            Err(anyhow!(
                "Can not match {line:?} with regular expression {REGEX:?}"
            ))
        }
    }

    fn parse_as_add_tags<'a, 'b>(
        line: &str,
        aliases_resolver: &mut dyn AliasesResolver,
    ) -> Result<Self> {
        static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let regex = REGEX.get_or_init(|| {
            Regex::new(r#"^/may +(\S+(?: +\S+)*) +tag +(\S+)$"#)
                .with_context(|| "Can not compile regular expression")
                .unwrap()
        });
        if let Some(captures) = regex.captures(line) {
            let tags_capture = &captures[1];
            let reference_capture = &captures[2];
            Ok(Self::AddTags {
                thesis_id: aliases_resolver
                    .get_thesis_id_by_reference(&Reference::new(reference_capture)?)?,
                tags: tags_capture
                    .split(' ')
                    .map(|tag_string| Tag(tag_string.to_string()))
                    .collect(),
            }
            .validated()?)
        } else {
            Err(anyhow!(
                "Can not match {line:?} with regular expression {REGEX:?}"
            ))
        }
    }

    fn parse_as_remove_tags<'a, 'b>(
        line: &str,
        aliases_resolver: &mut dyn AliasesResolver,
    ) -> Result<Self> {
        static REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
        let regex = REGEX.get_or_init(|| {
            Regex::new(r#"^/may +(\S+(?: +\S+)*) +not tag +(\S+)$"#)
                .with_context(|| "Can not compile regular expression")
                .unwrap()
        });
        if let Some(captures) = regex.captures(line) {
            let tags_capture = &captures[1];
            let reference_capture = &captures[2];
            Ok(Self::RemoveTags {
                thesis_id: aliases_resolver
                    .get_thesis_id_by_reference(&Reference::new(reference_capture)?)?,
                tags: tags_capture
                    .split(' ')
                    .map(|tag_string| Tag(tag_string.to_string()))
                    .collect(),
            }
            .validated()?)
        } else {
            Err(anyhow!(
                "Can not match {line:?} with regular expression {REGEX:?}"
            ))
        }
    }

    pub fn parse<'a, 'b>(
        line: &str,
        aliases_resolver: &mut dyn AliasesResolver,
        supported_relations_kinds: &BTreeSet<RelationKind>,
    ) -> Result<Self> {
        let mut errors = vec![];
        match Self::parse_as_set_alias(line, aliases_resolver) {
            Ok(result) => {
                return Ok(result);
            }
            Err(error) => {
                errors.push(("set alias", error));
            }
        }
        match Self::parse_as_add_relation_thesis_with_alias(
            line,
            aliases_resolver,
            supported_relations_kinds,
        ) {
            Ok(result) => {
                return Ok(result);
            }
            Err(error) => {
                errors.push(("add relation thesis with alias", error));
            }
        }
        match Self::parse_as_add_relation_thesis_without_alias(
            line,
            aliases_resolver,
            supported_relations_kinds,
        ) {
            Ok(result) => {
                return Ok(result);
            }
            Err(error) => {
                errors.push(("add relation thesis without alias", error));
            }
        }
        match Self::parse_as_add_text_thesis_with_alias(line, aliases_resolver) {
            Ok(result) => {
                return Ok(result);
            }
            Err(error) => {
                errors.push(("add text thesis with alias", error));
            }
        }
        match Self::parse_as_add_text_thesis_without_alias(line, aliases_resolver) {
            Ok(result) => {
                return Ok(result);
            }
            Err(error) => {
                errors.push(("add text thesis without alias", error));
            }
        }
        match Self::parse_as_add_tags(line, aliases_resolver) {
            Ok(result) => {
                return Ok(result);
            }
            Err(error) => {
                errors.push(("add tags", error));
            }
        }
        match Self::parse_as_remove_tags(line, aliases_resolver) {
            Ok(result) => {
                return Ok(result);
            }
            Err(error) => {
                errors.push(("remove tags", error));
            }
        }
        Err(anyhow!(
            "Can not parse command line {line:?}:\ncan not be parsed as {}",
            errors
                .into_iter()
                .map(|(command_name, result)| format!(
                    "{command_name} because {}",
                    result.to_string()
                ))
                .collect::<Vec<_>>()
                .join("\nand can not be parsed as ")
        ))
    }
}
