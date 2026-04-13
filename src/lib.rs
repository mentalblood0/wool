pub extern crate anyhow;
pub extern crate fallible_iterator;
pub extern crate html_escape;
pub extern crate regex;
pub extern crate serde;
pub extern crate trove;

pub use trove::bincode;
pub use trove::xxhash_rust;

#[macro_export]
macro_rules! define_sweater {
    ($sweater_name:ident(
        $(
            $bucket_name:ident
        )*
    ) use {
        $($use_item:tt)*
    }) => {
        pub mod $sweater_name {
            use {
                std::collections::{BTreeSet, BTreeMap},
                $crate::{
                    trove::{define_chest, path_segments, search_path_segments, DocumentId},
                    html_escape::encode_text,
                    bincode::{self, Encode, encode_to_vec, config},
                    fallible_iterator::FallibleIterator,
                    serde::{Deserialize, Serialize},
                    trove::Document,
                    anyhow::{anyhow, Context, Result, Error},
                    regex::Regex,
                },
            };

            define_chest!(chest(
                theses
                $(
                    $bucket_name
                )*
            ) {
            } {
            } use {
                $($use_item)*
            });

            #[derive(Serialize, Deserialize, Debug, Clone)]
            pub struct SweaterConfig {
                pub chest: chest::ChestConfig,
                pub supported_relations_kinds: BTreeSet<RelationKind>,
            }

            pub struct Sweater {
                pub chest: chest::Chest,
                pub config: SweaterConfig,
            }

            impl Sweater {
                pub fn new(config: SweaterConfig) -> Result<Self> {
                    Ok(Self {
                        chest: chest::Chest::new(config.chest.clone()).with_context(|| {
                            format!(
                                "Can not create sweater with chest config {:?}",
                                config.chest
                            )
                        })?,
                        config: config,
                    })
                }

                pub fn lock_all_and_write<'a, F, R>(&'a mut self, mut f: F) -> Result<R>
                where
                    F: FnMut(&mut WriteTransaction<'_, '_, '_, '_>) -> Result<R>,
                {
                    self.chest
                        .lock_all_and_write(|chest_write_transaction| {
                            f(&mut WriteTransaction {
                                chest_transaction: chest_write_transaction,
                                sweater_config: self.config.clone(),
                            })
                        })
                        .with_context(|| "Can not lock chest and initiate write transaction")
                }

                pub fn lock_all_writes_and_read<F, R>(&self, mut f: F) -> Result<R>
                where
                    F: FnMut(ReadTransaction) -> Result<R>,
                {
                    self.chest
                        .lock_all_writes_and_read(|chest_read_transaction| {
                            f(ReadTransaction {
                                chest_transaction: &chest_read_transaction,
                                sweater_config: &self.config,
                            })
                        })
                        .with_context(|| {
                            "Can not lock all write operations on chest and initiate read transaction"
                        })
                }
            }


            pub struct ReadTransaction<'a> {
                pub chest_transaction: &'a chest::ReadTransaction<'a>,
                pub sweater_config: &'a SweaterConfig,
            }

            macro_rules! define_read_methods {
                ($lifetime:lifetime) => {
                    fn get_thesis(&self, thesis_id: &DocumentId) -> Result<Option<Thesis>> {
                        if let Some(thesis_json_value) =
                            self.chest_transaction.theses_get(thesis_id, &vec![])?
                        {
                            Ok(Some(serde_json::from_value(thesis_json_value).unwrap()))
                        } else {
                            Ok(None)
                        }
                    }

                    fn iter_theses_ids_by_tags(
                        &self,
                        present_tags: &Vec<Tag>,
                        absent_tags: &Vec<Tag>,
                        start_after_thesis_id: Option<DocumentId>
                    ) -> Result<Box<dyn FallibleIterator<Item = DocumentId, Error = Error> + '_>> {
                        self
                            .chest_transaction
                            .theses_select(
                                &present_tags.iter().map(|tag| (search_path_segments!("tags", ()), serde_json::to_value(tag).unwrap())).collect::<Vec<_>>(),
                                &absent_tags.iter().map(|tag| (search_path_segments!("tags", ()), serde_json::to_value(tag).unwrap())).collect::<Vec<_>>(),
                                start_after_thesis_id,
                            )
                    }

                    fn get_thesis_id_by_alias(&self, alias: &Alias) -> Result<Option<DocumentId>> {
                        Ok(self
                            .chest_transaction
                            .theses_select(
                                &vec![(
                                    search_path_segments!("alias"),
                                    serde_json::to_value(alias)?,
                                )],
                                &vec![],
                                None,
                            )?
                            .next()?)
                    }

                    fn where_referenced(&self, thesis_id: &DocumentId) -> Result<Vec<DocumentId>> {
                        let json_value = serde_json::to_value(thesis_id)?;
                        self.chest_transaction
                            .theses_select(
                                &vec![(
                                    search_path_segments!("content", "Text", "references", ()),
                                    json_value.clone(),
                                )],
                                &vec![],
                                None,
                            )?
                            .chain(self.chest_transaction.theses_select(
                                &vec![(
                                    search_path_segments!("content", "Relation", "from"),
                                    json_value.clone(),
                                )],
                                &vec![],
                                None,
                            )?)
                            .chain(self.chest_transaction.theses_select(
                                &vec![(
                                    search_path_segments!("content", "Relation", "to"),
                                    json_value,
                                )],
                                &vec![],
                                None,
                            )?)
                            .collect()
                    }

                    fn get_alias_by_thesis_id(&self, thesis_id: &DocumentId) -> Result<Option<Alias>> {
                        Ok(
                            if let Some(json_value) = self
                                .chest_transaction
                                .theses_get(thesis_id, &path_segments!("alias"))?
                            {
                                serde_json::from_value(json_value)?
                            } else {
                                None
                            },
                        )
                    }

                    fn iter_theses(
                        &self,
                    ) -> Result<Box<dyn FallibleIterator<Item = Thesis, Error = Error> + '_>> {
                        Ok(Box::new(self.chest_transaction.theses_documents()?.map(
                            |document| Ok(serde_json::from_value(document.value)?),
                        )))
                    }
                };
            }

            pub trait ReadTransactionMethods<'a> {
                fn get_thesis(&self, thesis_id: &DocumentId) -> Result<Option<Thesis>>;
                fn iter_theses_ids_by_tags(
                    &self,
                    present_tags: &Vec<Tag>,
                    absent_tags: &Vec<Tag>,
                    start_after_thesis_id: Option<DocumentId>
                ) -> Result<Box<dyn FallibleIterator<Item = DocumentId, Error = Error> + '_>>;
                fn get_thesis_id_by_alias(&self, alias: &Alias) -> Result<Option<DocumentId>>;
                fn get_alias_by_thesis_id(&self, thesis_id: &DocumentId) -> Result<Option<Alias>>;
                fn where_referenced(&self, thesis_id: &DocumentId) -> Result<Vec<DocumentId>>;
                fn iter_theses(&self) -> Result<Box<dyn FallibleIterator<Item = Thesis, Error = Error> + '_>>;
            }

            impl<'a> ReadTransactionMethods<'a> for ReadTransaction<'a> {
                define_read_methods!('a);
            }


            pub struct WriteTransaction<'a, 'b, 'c, 'd> {
                pub chest_transaction: &'a mut chest::WriteTransaction<'b, 'c, 'd>,
                pub sweater_config: SweaterConfig,
            }

            impl<'a, 'b, 'c, 'd> ReadTransactionMethods<'a> for WriteTransaction<'a, 'b, 'c, 'd> {
                define_read_methods!('a);
            }

            impl<'a, 'b, 'c, 'd> ReadTransactionMethods<'a> for &mut WriteTransaction<'a, 'b, 'c, 'd> {
                define_read_methods!('a);
            }

            impl WriteTransaction<'_, '_, '_, '_> {
                pub fn insert_thesis(&mut self, thesis: Thesis) -> Result<()> {
                    let thesis_id = thesis.id()?;
                    if self
                        .chest_transaction
                        .theses_contains_document_with_id(&thesis_id)?
                    {
                        Err(anyhow!(
                            "Can not insert thesis {thesis:?} with id {thesis_id:?} as chest already contains \
                             document with such id"
                        ))
                    } else {
                        if let Content::Relation(Relation {
                            from: ref from_id,
                            to: ref to_id,
                            kind: ref relation_kind,
                        }) = thesis.content
                        {
                            if !self
                                .sweater_config
                                .supported_relations_kinds
                                .contains(&relation_kind)
                            {
                                return Err(anyhow!(
                                    "Can not insert relation {thesis:?} of kind {relation_kind:?} in sweater \
                                     with supported relations kinds {:?} as it's kind is not supported",
                                    self.sweater_config.supported_relations_kinds
                                ));
                            }
                            for (name, related_id) in [("from", from_id), ("to", to_id)] {
                                if self
                                    .chest_transaction
                                    .theses_get(&related_id, &path_segments!("content"))?
                                    .is_none()
                                {
                                    return Err(anyhow!(
                                        "Can not insert relation {thesis:?} in sweater without inserted \
                                         {name:?} thesis with {related_id:?}"
                                    ));
                                }
                            }
                        }
                        self.chest_transaction.theses_insert_with_id(Document {
                            id: thesis_id,
                            value: serde_json::to_value(thesis.clone())?,
                        })?;
                        Ok(())
                    }
                }

                pub fn add_tags(&mut self, thesis_id: &DocumentId, tags: Vec<Tag>) -> Result<()> {
                    let new_tags = fallible_iterator::convert(
                        tags.into_iter().map(|tag| Ok::<_, anyhow::Error>(tag))
                    ).map(|tag| serde_json::to_value(tag).map_err(Into::into))
                    .filter(
                        |tag_json| Ok(!self.chest_transaction.theses_contains_element(
                            thesis_id,
                            &search_path_segments!("tags", ()),
                            tag_json
                        )?)
                    )
                    .collect::<Vec<_>>()?;
                        self.chest_transaction.theses_push(
                            thesis_id,
                            path_segments!("tags"),
                            new_tags,
                        )?;
                    Ok(())
                }

                pub fn remove_tags(&mut self, thesis_id: &DocumentId, tags: &Vec<Tag>) -> Result<()> {
                    for tag in tags {
                        if let Some(tag_index_in_array) = self.chest_transaction.theses_get_element_index(
                            thesis_id,
                            &search_path_segments!("tags", ()),
                            &serde_json::to_value(tag)?,
                        )? {
                            self.chest_transaction
                                .theses_remove(thesis_id, &path_segments!("tags", tag_index_in_array))?;
                        }
                    }
                    Ok(())
                }

                pub fn remove_thesis(&mut self, thesis_id: &DocumentId) -> Result<()> {
                    if self
                        .chest_transaction
                        .theses_contains_document_with_id(thesis_id)?
                    {
                        self.chest_transaction.theses_remove(thesis_id, &vec![])?;
                        let thesis_id_json_value = serde_json::to_value(thesis_id)?;
                        let relations_ids = self
                            .chest_transaction
                            .theses_select(
                                &vec![(
                                    search_path_segments!("content", "Relation", "from", ()),
                                    thesis_id_json_value.clone(),
                                )],
                                &vec![],
                                None,
                            )?
                            .chain(self.chest_transaction.theses_select(
                                &vec![(
                                    search_path_segments!("content", "Relation", "to", ()),
                                    thesis_id_json_value,
                                )],
                                &vec![],
                                None,
                            )?)
                            .collect::<Vec<_>>()?;
                        for relation_id in relations_ids {
                            self.chest_transaction
                                .theses_remove(&relation_id, &vec![])?;
                        }
                        let where_mentioned = self.where_referenced(thesis_id)?;
                        for id_of_thesis_where_mentioned in where_mentioned {
                            self.remove_thesis(&id_of_thesis_where_mentioned)?;
                        }
                    }
                    Ok(())
                }

                pub fn set_alias(&mut self, thesis_id: DocumentId, new_alias: Alias) -> Result<()> {
                    self.chest_transaction.theses_set(
                        thesis_id,
                        path_segments!("alias"),
                        serde_json::to_value(new_alias)?,
                    )?;
                    Ok(())
                }
            }

            #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
            pub struct Alias(pub String);

            impl Alias {
                pub fn validated(&self) -> Result<&Self> {
                    static ALIAS_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
                    let sentence_regex = ALIAS_REGEX.get_or_init(|| {
                        Regex::new(r#"^[^ \[\]]+$"#)
                            .with_context(|| "Can not compile regular expression for thesis alias validation")
                            .unwrap()
                    });
                    if sentence_regex.is_match(&self.0) {
                        Ok(self)
                    } else {
                        Err(anyhow!(
                            "Alias must be sequence of one or more non-whitespace characters except '[' and ']', so {:?} does \
                             not seem to be alias",
                            self.0
                        ))
                    }
                }
            }

            #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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

            pub struct AliasesResolver<'a> {
                pub read_able_transaction: &'a dyn ReadTransactionMethods<'a>,
                pub known_aliases: BTreeMap<Alias, DocumentId>,
            }

            impl<'a> AliasesResolver<'a> {
                pub fn get_thesis_id_by_reference(&self, reference: &Reference) -> Result<DocumentId> {
                    Ok(match reference {
                        Reference::DocumentId(thesis_id) => {
                            if self.read_able_transaction.get_thesis(thesis_id)?.is_none() {
                                return Err(anyhow!("Can not find thesis with id {thesis_id:?}"));
                            }
                            thesis_id.clone()
                        }
                        Reference::Alias(alias) => {
                            if let Some(result) = self.known_aliases.get(alias) {
                                result.clone()
                            } else {
                                self.read_able_transaction
                                    .get_thesis_id_by_alias(alias)?
                                    .ok_or_else(|| anyhow!("Can not find thesis id by alias {alias:?}"))?
                            }
                        }
                    })
                }

                pub fn remember(&mut self, alias: Alias, document_id: DocumentId) -> &Self {
                    self.known_aliases.insert(alias, document_id);
                    self
                }
            }

            #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
            pub struct Thesis {
                pub alias: Option<Alias>,
                pub content: Content,

                #[serde(default)]
                pub tags: Vec<Tag>,
            }

            impl Thesis {
                pub fn id(&self) -> Result<DocumentId> {
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
            }


            #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
            pub struct Text {
                #[serde(default)]
                pub raw_text_parts: Vec<RawText>,
                #[serde(default)]
                pub references: Vec<DocumentId>,
                pub start_with_reference: bool,
            }

            impl<'a> Text {
                pub fn new(input: &str, aliases_resolver: &mut AliasesResolver) -> Result<Self> {
                    static REFERENCE_IN_TEXT_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
                    let reference_in_text_regex = REFERENCE_IN_TEXT_REGEX.get_or_init(|| {
                        Regex::new(r#"\[(:?([A-Za-z0-9-_]{22})|([^\[\]]+))\]"#)
                            .with_context(|| {
                                "Can not compile regular expression to split text on raw text parts and \
                                 references"
                            })
                            .unwrap()
                    });

                    let mut result = Self {
                        raw_text_parts: Vec::new(),
                        references: Vec::new(),
                        start_with_reference: false,
                    };
                    let mut last_match_end = 0;
                    for reference_match in reference_in_text_regex.captures_iter(input) {
                        let full_reference_match = reference_match.get(0).unwrap();
                        if full_reference_match.start() == 0 {
                            result.start_with_reference = true;
                        }
                        let text_before = &input[last_match_end..full_reference_match.start()];
                        if !text_before.is_empty() {
                            result.raw_text_parts.push(RawText(text_before.to_string()));
                        }
                        if let Some(thesis_id_string) = reference_match
                            .get(2)
                            .map(|thesis_id_string_match| thesis_id_string_match.as_str())
                        {
                            result.references.push(
                                serde_json::from_value(serde_json::Value::String(thesis_id_string.to_string()))
                                    .unwrap(),
                            );
                        } else if let Some(alias_string) = reference_match
                            .get(3)
                            .map(|alias_string_match| alias_string_match.as_str())
                        {
                            result.references.push(
                                aliases_resolver
                                    .get_thesis_id_by_reference(&Reference::Alias(Alias(
                                        alias_string.to_string(),
                                    ).validated()?.to_owned()))
                                    .with_context(|| {
                                        anyhow!(
                                            "Can not parse text {:?} with alias {:?} because do not know such \
                                             alias",
                                            input,
                                            alias_string
                                        )
                                    })?,
                            );
                        }
                        last_match_end = full_reference_match.end();
                    }
                    if last_match_end < input.len() {
                        let remaining = &input[last_match_end..];
                        if !remaining.is_empty() {
                            result.raw_text_parts.push(RawText(remaining.to_string()));
                        }
                    }

                    Ok(result)
                }

                pub fn composed<F>(&self, format_reference: F) -> Result<String>
                where F: Fn(&DocumentId) -> Result<String> {
                    let mut result_list = Vec::new();
                    if self.start_with_reference {
                        for (reference_index, reference) in self.references.iter().enumerate() {
                            result_list.push(format!(
                                "[{}]",
                                format_reference(reference)?
                            ));
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
                    self.composed(
                        |referenced_thesis_id|
                            Ok(serde_json::to_value(referenced_thesis_id).unwrap().as_str().unwrap().to_string())
                    ).unwrap()
                }

                pub fn composed_with_aliases(
                    &self,
                    read_able_transaction: &dyn ReadTransactionMethods<'a>,
                ) -> Result<String> {
                    self.composed(|reference|
                                Ok(if let Some(alias) = read_able_transaction.get_alias_by_thesis_id(reference)? {
                                    alias.0
                                } else {
                                    reference.to_string()
                                }))
                }

                pub fn validated(&self) -> Result<&Self> {
                    for part in self.raw_text_parts.iter() {
                        part.validated()?;
                    }
                    Ok(self)
                }
            }

            #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
            pub enum Content {
                Text(Text),
                Relation(Relation),
            }

            impl Content {
                pub fn id(&self) -> Result<DocumentId> {
                    let source = match self {
                        Content::Text(text) => text.composed_raw().bytes().collect(),
                        Content::Relation(relation) => {
                            encode_to_vec(relation, config::standard()).with_context(
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
                        value: $crate::xxhash_rust::xxh3::xxh3_128(&source).to_be_bytes(),
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

            #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
            pub struct Tag(pub String);

            impl Tag {
                pub fn validated(&self) -> Result<&Self> {
                    static TAG_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
                    let tag_regex = TAG_REGEX.get_or_init(|| {
                        Regex::new(r"^\w+$")
                            .with_context(|| "Can not compile regular expression for tag validation")
                            .unwrap()
                    });
                    if tag_regex.is_match(&self.0) {
                        Ok(self)
                    } else {
                        Err(anyhow!(
                            "Tag must be a word characters sequence, so {:?} does not seem to be tag",
                            self.0
                        ))
                    }
                }
            }

            #[derive(Serialize, Deserialize, Debug, Clone, Encode, PartialEq, Eq, PartialOrd, Ord)]
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
                            "Relation kind must be an English words sequence without punctuation, so {:?} \
                             does not seem to be relation kind",
                            self.0
                        ))
                    }
                }
            }

            #[derive(Serialize, Deserialize, Debug, Clone, Encode, PartialEq, Eq)]
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

            #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
            pub enum Reference {
                Alias(Alias),
                DocumentId(DocumentId),
            }

            impl Reference {
                pub fn new(input: &str) -> Result<Self> {
                    if let Ok(document_id) = serde_json::from_value::<DocumentId>(
                        serde_json::Value::String(input.to_string()),
                    ) {
                        Ok(Self::DocumentId(document_id))
                    } else {
                        Ok(Self::Alias(Alias(input.to_string()).validated()?.to_owned()))
                    }
                }
            }

            #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
            pub enum Command {
                AddTextThesisWithAlias(Thesis),
                AddTextThesisWithoutAlias(Thesis),
                AddRelationThesisWithAlias(Thesis),
                AddRelationThesisWithoutAlias(Thesis),
                SetAlias { thesis_id: DocumentId, alias: Alias },
                AddTags { thesis_id: DocumentId, tags: Vec<Tag> },
                RemoveTags { thesis_id: DocumentId, tags: Vec<Tag> }
            }

            impl Command {
                pub fn validated(self) -> Result<Self> {
                    match self {
                        Command::AddTextThesisWithAlias(ref thesis) => { thesis.validated()?; }
                        Command::AddTextThesisWithoutAlias(ref thesis) => { thesis.validated()?; }
                        Command::AddRelationThesisWithAlias(ref thesis) => { thesis.validated()?; }
                        Command::AddRelationThesisWithoutAlias(ref thesis) => { thesis.validated()?; }
                        Command::SetAlias { ref alias, .. } => { alias.validated()?; }
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
                    aliases_resolver: &'b mut AliasesResolver<'a>,
                ) -> Result<Self> {
                    static REGEX: std::sync::OnceLock<Regex> =
                        std::sync::OnceLock::new();
                    let regex = REGEX.get_or_init(|| {
                        Regex::new(r#"^/may +(\S+) +alias +(\S+)$"#)
                            .with_context(|| "Can not compile regular expression").unwrap()
                    });
                    if let Some(captures) = regex.captures(line) {
                        let alias_capture = &captures[1];
                        let reference_capture = &captures[2];
                        let alias = Alias(alias_capture.to_string()).validated()?.to_owned();
                        let thesis_id = aliases_resolver.get_thesis_id_by_reference(&Reference::new(&reference_capture)?)?;
                        let result = Self::SetAlias { thesis_id: thesis_id.clone(), alias: alias.clone() }.validated()?;
                        aliases_resolver.remember(alias, thesis_id);
                        Ok(result)
                    } else {
                        Err(anyhow!("Can not match {line:?} with regular expression {REGEX:?}"))
                    }
                }

                fn parse_as_add_relation_thesis_with_alias<'a, 'b>(
                    line: &str,
                    aliases_resolver: &'b mut AliasesResolver<'a>,
                    supported_relations_kinds: &BTreeSet<RelationKind>
                ) -> Result<Self> {
                    static REGEX: std::sync::OnceLock<Regex> =
                        std::sync::OnceLock::new();
                    let regex = REGEX.get_or_init(|| {
                        Regex::new(r#"^/may +(\S+) alias +(\S+) +(.+) +(\S+)$"#)
                            .with_context(|| "Can not compile regular expression").unwrap()
                    });
                    if let Some(captures) = regex.captures(line) {
                        let alias_capture = &captures[1];
                        let from_reference_capture = &captures[2];
                        let relation_kind_capture = &captures[3];
                        let to_reference_capture = &captures[4];
                        let relation_kind = RelationKind(relation_kind_capture.to_string());
                        if !supported_relations_kinds.contains(&relation_kind) {
                            return Err(anyhow!("Relation kind {relation_kind:?} is not supported"))
                        }
                        let alias = Alias(alias_capture.to_string()).validated()?.to_owned();
                        let thesis = Thesis {
                            alias: Some(alias.clone()),
                            content: Content::Relation(Relation {
                                from: aliases_resolver.get_thesis_id_by_reference(&Reference::new(from_reference_capture)?)?,
                                kind: relation_kind,
                                to: aliases_resolver.get_thesis_id_by_reference(&Reference::new(to_reference_capture)?)?,
                            }),
                            tags: vec![]
                        };
                        let id = thesis.id()?;
                        let result = Self::AddRelationThesisWithAlias(thesis).validated()?;
                        aliases_resolver.remember(alias, id);
                        Ok(result)
                    } else {
                        Err(anyhow!("Can not match {line:?} with regular expression {REGEX:?}"))
                    }
                }

                fn parse_as_add_relation_thesis_without_alias<'a, 'b>(
                    line: &str,
                    aliases_resolver: &'b mut AliasesResolver<'a>,
                    supported_relations_kinds: &BTreeSet<RelationKind>
                ) -> Result<Self> {
                    static REGEX: std::sync::OnceLock<Regex> =
                        std::sync::OnceLock::new();
                    let regex = REGEX.get_or_init(|| {
                        Regex::new(r#"^/may +(\S+) +(.+) +(\S+)$"#)
                            .with_context(|| "Can not compile regular expression").unwrap()
                    });
                    if let Some(captures) = regex.captures(line) {
                        let from_reference_capture = &captures[1];
                        let relation_kind_capture = &captures[2];
                        let to_reference_capture = &captures[3];
                        let relation_kind = RelationKind(relation_kind_capture.to_string());
                        if !supported_relations_kinds.contains(&relation_kind) {
                            return Err(anyhow!("Relation kind {relation_kind:?} is not supported"))
                        }
                        let to = aliases_resolver.get_thesis_id_by_reference(&Reference::new(to_reference_capture)?)?;
                        let result = Self::AddRelationThesisWithoutAlias( Thesis {
                            alias: None,
                            content: Content::Relation(Relation {
                                from: aliases_resolver.get_thesis_id_by_reference(&Reference::new(from_reference_capture)?)?,
                                kind: relation_kind,
                                to: to.clone()
                            }),
                            tags: vec![]
                        }).validated()?;
                        Ok(result)
                    } else {
                        Err(anyhow!("Can not match {line:?} with regular expression {REGEX:?}"))
                    }
                }

                fn parse_as_add_text_thesis_with_alias<'a, 'b>(
                    line: &str,
                    aliases_resolver: &'b mut AliasesResolver<'a>,
                ) -> Result<Self> {
                    static REGEX: std::sync::OnceLock<Regex> =
                        std::sync::OnceLock::new();
                    let regex = REGEX.get_or_init(|| {
                        Regex::new(r#"^/may +(\S+) +alias +(.+)$"#)
                            .with_context(|| "Can not compile regular expression").unwrap()
                    });
                    if let Some(captures) = regex.captures(line) {
                        let alias_capture = &captures[1];
                        let thesis_text_capture = &captures[2];
                        let alias = Alias(alias_capture.to_string()).validated()?.to_owned();
                        let thesis = Thesis {
                            alias: Some(alias.clone()),
                            content: Content::Text(Text::new(&thesis_text_capture, aliases_resolver)?),
                            tags: vec![]
                        };
                        aliases_resolver.remember(alias, thesis.id()?);
                        Self::AddTextThesisWithAlias(thesis).validated()
                    } else {
                        Err(anyhow!("Can not match {line:?} with regular expression {REGEX:?}"))
                    }
                }

                fn parse_as_add_text_thesis_without_alias<'a, 'b>(
                    line: &str,
                    aliases_resolver: &'b mut AliasesResolver<'a>,
                ) -> Result<Self> {
                    static REGEX: std::sync::OnceLock<Regex> =
                        std::sync::OnceLock::new();
                    let regex = REGEX.get_or_init(|| {
                        Regex::new(r#"^/may +(.+)$"#)
                            .with_context(|| "Can not compile regular expression").unwrap()
                    });
                    if let Some(captures) = regex.captures(line) {
                        let thesis_text_capture = &captures[1];
                        Self::AddTextThesisWithoutAlias(Thesis {
                            alias: None,
                            content: Content::Text(Text::new(thesis_text_capture, aliases_resolver)?),
                            tags: vec![]
                        }).validated()
                    } else {
                        Err(anyhow!("Can not match {line:?} with regular expression {REGEX:?}"))
                    }
                }

                fn parse_as_add_tags<'a, 'b>(
                    line: &str,
                    aliases_resolver: &'b mut AliasesResolver<'a>,
                ) -> Result<Self> {
                    static REGEX: std::sync::OnceLock<Regex> =
                        std::sync::OnceLock::new();
                    let regex = REGEX.get_or_init(|| {
                        Regex::new(r#"^/may +(\S+(?: +\S+)*) +tag +(\S+)$"#)
                            .with_context(|| "Can not compile regular expression").unwrap()
                    });
                    if let Some(captures) = regex.captures(line) {
                        let tags_capture = &captures[1];
                        let reference_capture = &captures[2];
                        Ok(Self::AddTags {
                            thesis_id: aliases_resolver.get_thesis_id_by_reference(&Reference::new(reference_capture)?)?,
                            tags: tags_capture
                                .split(' ')
                                .map(|tag_string| Tag(tag_string.to_string()))
                                .collect(),
                        }.validated()?)
                    } else {
                        Err(anyhow!("Can not match {line:?} with regular expression {REGEX:?}"))
                    }
                }

                fn parse_as_remove_tags<'a, 'b>(
                    line: &str,
                    aliases_resolver: &'b mut AliasesResolver<'a>,
                ) -> Result<Self> {
                    static REGEX: std::sync::OnceLock<Regex> =
                        std::sync::OnceLock::new();
                    let regex = REGEX.get_or_init(|| {
                        Regex::new(r#"^/may +(\S+(?: +\S+)*) +not tag +(\S+)$"#)
                            .with_context(|| "Can not compile regular expression").unwrap()
                    });
                    if let Some(captures) = regex.captures(line) {
                        let tags_capture = &captures[1];
                        let reference_capture = &captures[2];
                        Ok(Self::RemoveTags {
                            thesis_id: aliases_resolver.get_thesis_id_by_reference(&Reference::new(reference_capture)?)?,
                            tags: tags_capture
                                .split(' ')
                                .map(|tag_string| Tag(tag_string.to_string()))
                                .collect(),
                        }.validated()?)
                    } else {
                        Err(anyhow!("Can not match {line:?} with regular expression {REGEX:?}"))
                    }
                }

                pub fn parse<'a, 'b>(
                    line: &str,
                    aliases_resolver: &'b mut AliasesResolver<'a>,
                    supported_relations_kinds: &BTreeSet<RelationKind>
                ) -> Result<Self> {
                    let mut errors = vec![];
                    match Self::parse_as_set_alias(line, aliases_resolver) {
                        Ok(result) => { return Ok(result); }
                        Err(error) => { errors.push(("set alias", error)); }
                    }
                    match Self::parse_as_add_relation_thesis_with_alias(line, aliases_resolver, supported_relations_kinds) {
                        Ok(result) => { return Ok(result); }
                        Err(error) => { errors.push(("add relation thesis with alias", error)); }
                    }
                    match Self::parse_as_add_relation_thesis_without_alias(line, aliases_resolver, supported_relations_kinds) {
                        Ok(result) => { return Ok(result); }
                        Err(error) => { errors.push(("add relation thesis without alias", error)); }
                    }
                    match Self::parse_as_add_text_thesis_with_alias(line, aliases_resolver) {
                        Ok(result) => { return Ok(result); }
                        Err(error) => { errors.push(("add text thesis with alias", error)); }
                    }
                    match Self::parse_as_add_text_thesis_without_alias(line, aliases_resolver) {
                        Ok(result) => { return Ok(result); }
                        Err(error) => { errors.push(("add text thesis without alias", error)); }
                    }
                    match Self::parse_as_add_tags(line, aliases_resolver) {
                        Ok(result) => { return Ok(result); }
                        Err(error) => { errors.push(("add tags", error)); }
                    }
                    match Self::parse_as_remove_tags(line, aliases_resolver) {
                        Ok(result) => { return Ok(result); }
                        Err(error) => { errors.push(("remove tags", error)); }
                    }
                    Err(anyhow!("Can not parse command line {line:?}:\ncan not be parsed as {}",
                        errors.into_iter().map(|(command_name, result)| format!("{command_name} because {}", result.to_string())).collect::<Vec<_>>().join("\nand can not be parsed as ")))
                }

                pub fn execute(&self, transaction: &mut WriteTransaction) -> Result<()> {
                    match self {
                        Command::AddTextThesisWithAlias(thesis) => { transaction.insert_thesis(thesis.clone()) }
                        Command::AddTextThesisWithoutAlias(thesis) => { transaction.insert_thesis(thesis.clone()) }
                        Command::AddRelationThesisWithAlias(thesis) => { transaction.insert_thesis(thesis.clone()) }
                        Command::AddRelationThesisWithoutAlias(thesis) => { transaction.insert_thesis(thesis.clone()) }
                        Command::SetAlias { thesis_id, alias } => { transaction.set_alias(thesis_id.clone(), alias.clone()) }
                        Command::AddTags { thesis_id, tags } => { transaction.add_tags(thesis_id, tags.clone()) }
                        Command::RemoveTags { thesis_id, tags } => { transaction.remove_tags(thesis_id, &tags) }
                    }
                }
            }

            #[derive(PartialEq, Eq, Serialize, Deserialize)]
            pub enum ExternalizeRelationsNodes {
                None,
                Related,
                All,
            }

            #[derive(PartialEq, Eq, Serialize, Deserialize)]
            pub enum ShowNodesReferences {
                None,
                Mentioned,
                All,
            }

            #[derive(Serialize, Deserialize)]
            pub struct GraphGeneratorConfig {
                pub wrap_width: u16,
            }

            pub enum Stage {
                BeforeFirstLine,
                Middle,
                AfterLastLine,
            }

            pub struct GraphGenerator<'a> {
                pub config: &'a GraphGeneratorConfig,
                pub read_able_transaction: &'a dyn ReadTransactionMethods<'a>,
                pub theses_iterator: Box<dyn FallibleIterator<Item = Thesis, Error = Error> + 'a>,
                pub stage: Stage,
            }

            impl<'a> GraphGenerator<'a> {
                pub fn new(
                    config: &'a GraphGeneratorConfig,
                    read_able_transaction: &'a dyn ReadTransactionMethods<'a>,
                ) -> Result<Self> {
                    Ok(Self {
                        config,
                        read_able_transaction,
                        theses_iterator: Box::new(read_able_transaction.iter_theses()?),
                        stage: Stage::BeforeFirstLine,
                    })
                }
            }

            impl<'a> GraphGenerator<'a> {
                fn wrap(&self, text: &str) -> String {
                    let wrap_width = self.config.wrap_width as usize;
                    if wrap_width == 0 {
                        return String::new();
                    }

                    let mut result = String::with_capacity(text.len() + (text.len() / wrap_width) * 5);
                    let mut current_line = String::new();
                    let mut current_line_size = 0;
                    let mut first_line = true;

                    for word in text.split_whitespace() {
                        let word_size = word.len();

                        if current_line.is_empty() {
                            current_line.reserve(word_size);
                            current_line.push_str(word);
                            current_line_size = word_size;
                        } else if current_line_size + 1 + word_size <= wrap_width {
                            current_line.push(' ');
                            current_line.push_str(word);
                            current_line_size += 1 + word_size;
                        } else {
                            if !first_line {
                                result.push_str("<br/>");
                            }
                            result.push_str(&current_line);
                            first_line = false;
                            current_line = String::with_capacity(word_size);
                            current_line.push_str(word);
                            current_line_size = word_size;
                        }
                    }

                    if !current_line.is_empty() {
                        if !first_line {
                            result.push_str("<br/>");
                        }
                        result.push_str(&current_line);
                    }

                    result
                }
            }

            impl<'a> FallibleIterator for GraphGenerator<'a> {
                type Item = String;
                type Error = Error;

                fn next(&mut self) -> Result<Option<Self::Item>> {
                    Ok(match self.stage {
                        Stage::BeforeFirstLine => {
                            self.stage = Stage::Middle;
                            Some("digraph sweater {".to_string())
                        }
                        Stage::Middle => {
                            if let Some(thesis) = self.theses_iterator.next()? {
                                let thesis_id_string = thesis.id()?.to_string();
                                let node_header_text = if let Some(ref alias) = thesis.alias {
                                    encode_text(&alias.0).to_string()
                                } else {
                                    thesis_id_string.clone()
                                };
                                match thesis.content {
                                    Content::Text(ref text) => {
                                        let node_body_text =
                                            self.wrap(&text.composed_with_aliases(self.read_able_transaction)?);
                                        let node_header = format!(
                                            r#"<TR><TD BORDER="1" SIDES="b">{node_header_text}</TD></TR>"#,
                                        );
                                        let node_label = format!(
                                            r#"<TABLE BORDER="2" CELLSPACING="0" CELLPADDING="8">{}<TR><TD BORDER="0">{}</TD></TR></TABLE>"#,
                                            node_header, node_body_text
                                        );
                                        Some(
                                            format!(
                                                "\n\t\"{}\" [label=<{}>, shape=plaintext];", // node definition
                                                thesis_id_string, node_label
                                            ) + &thesis // node references arrows definitions
                                                .references()
                                                .iter()
                                                .map(|referenced_thesis_id| {
                                                    format!(
                                                        "\n\t\"{thesis_id_string}\" -> \"{}\" \
                                                         [arrowhead=none, color=\"grey\" style=dotted];",
                                                        referenced_thesis_id.to_string()
                                                    )
                                                })
                                                .collect::<Vec<_>>()
                                                .join(""),
                                        )
                                    }
                                    Content::Relation(ref relation) => {
                                        let node_label = format!(
                                            r#"<TABLE CELLSPACING="0" CELLPADDING="8" STYLE="dashed"><TR><TD SIDES="b" STYLE="dashed">{node_header_text}</TD></TR><TR><TD BORDER="0">{}</TD></TR></TABLE>"#,
                                            relation.kind.0
                                        );
                                        Some(format!(
                                            "\n\t\"{thesis_id_string}\" [label=<{node_label}>, \
                                             shape=plaintext];\n\t\"{}\" -> \"{}\" [dir=back, \
                                             arrowtail=tee];\n\t\"{}\" -> \"{}\";",
                                            relation.from.to_string(), // arrow to relation node
                                            thesis_id_string,
                                            thesis_id_string, // arrow from relation node
                                            relation.to.to_string()
                                        ))
                                    }
                                }
                            } else {
                                self.stage = Stage::AfterLastLine;
                                Some("\n}".to_string())
                            }
                        }
                        Stage::AfterLastLine => None,
                    })
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    define_sweater!(test_sweater(
        users
    ) use {
    });

    use {
        fallible_iterator::FallibleIterator,
        nanorand::{Rng, WyRand},
        pretty_assertions::assert_eq,
        std::collections::BTreeMap,
        test_sweater::*,
        trove::DocumentId,
    };

    fn new_default_sweater(test_name_for_isolation: &str) -> Sweater {
        Sweater::new(
            serde_saphyr::from_str(
                &std::fs::read_to_string("src/test_sweater_config.yml")
                    .unwrap()
                    .replace("TEST_NAME", test_name_for_isolation),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn random_text(
        rng: &mut WyRand,
        previously_added_theses: &BTreeMap<DocumentId, Thesis>,
        aliases_resolver: &mut AliasesResolver,
    ) -> Text {
        const ENGLISH_LETTERS: [&str; 26] = [
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q",
            "r", "s", "t", "u", "v", "w", "x", "y", "z",
        ];
        const RUSSIAN_LETTERS: [&str; 33] = [
            "а", "б", "в", "г", "д", "е", "ё", "ж", "з", "и", "й", "к", "л", "м", "н", "о", "п",
            "р", "с", "т", "у", "ф", "х", "ц", "ч", "ш", "щ", "ъ", "ы", "ь", "э", "ю", "я",
        ];
        const PUNCTUATION: &[&str] = &[",-'\""];
        let language = rng.generate_range(1..=2);
        let mut references_count = 0;
        let words: Vec<String> = (0..rng.generate_range(3..=10))
            .map(|_| {
                if previously_added_theses.is_empty() || rng.generate_range(0..=3) > 0 {
                    (0..rng.generate_range(2..=8))
                        .map(|_| {
                            if language == 1 {
                                ENGLISH_LETTERS[rng.generate_range(0..ENGLISH_LETTERS.len())]
                            } else {
                                RUSSIAN_LETTERS[rng.generate_range(0..RUSSIAN_LETTERS.len())]
                            }
                        })
                        .collect()
                } else {
                    references_count += 1;
                    format!(
                        "[{}]",
                        serde_json::to_value(
                            previously_added_theses
                                .keys()
                                .nth(rng.generate_range(0..previously_added_theses.len()))
                                .unwrap()
                                .clone(),
                        )
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string()
                    )
                }
            })
            .collect();
        let mut result_string = String::new();
        for (i, word) in words.iter().enumerate() {
            result_string.push_str(word);
            if i < words.len() - 1 {
                if rng.generate_range(0..3) == 0 {
                    result_string.push_str(PUNCTUATION[rng.generate_range(0..PUNCTUATION.len())]);
                } else {
                    result_string.push(' ');
                }
            }
        }
        let result = Text::new(&result_string, aliases_resolver).unwrap();
        assert_eq!(result.composed_raw(), result_string);
        result
    }

    fn random_tag(rng: &mut WyRand) -> Tag {
        const LETTERS: [&str; 26] = [
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q",
            "r", "s", "t", "u", "v", "w", "x", "y", "z",
        ];
        Tag((0..rng.generate_range(1..=2))
            .map(|_| LETTERS[rng.generate_range(0..LETTERS.len())])
            .collect())
    }

    fn random_thesis(
        rng: &mut WyRand,
        aliases_resolver: &mut AliasesResolver,
        previously_added_theses: &BTreeMap<DocumentId, Thesis>,
        transaction: &WriteTransaction,
    ) -> Thesis {
        let mut tags = (0..rng.generate_range(0..10))
            .map(|_| random_tag(rng))
            .collect::<Vec<_>>();
        tags.sort();
        tags.dedup();
        Thesis {
            alias: None,
            content: {
                let action_id = if previously_added_theses.is_empty() {
                    1
                } else {
                    rng.generate_range(1..=2)
                };
                match action_id {
                    1 => Content::Text(random_text(rng, previously_added_theses, aliases_resolver)),
                    2 => Content::Relation(Relation {
                        from: previously_added_theses
                            .keys()
                            .nth(rng.generate_range(0..previously_added_theses.len()))
                            .unwrap()
                            .clone(),
                        to: previously_added_theses
                            .keys()
                            .nth(rng.generate_range(0..previously_added_theses.len()))
                            .unwrap()
                            .clone(),
                        kind: transaction
                            .sweater_config
                            .supported_relations_kinds
                            .iter()
                            .nth(rng.generate_range(
                                0..transaction.sweater_config.supported_relations_kinds.len(),
                            ))
                            .unwrap()
                            .clone(),
                    }),
                    _ => {
                        panic!()
                    }
                }
            },
            tags: tags,
        }
    }

    #[test]
    fn test_generative() {
        let mut sweater = new_default_sweater("test_generative");
        let mut rng = WyRand::new_seed(0);

        sweater
            .lock_all_and_write(|transaction| {
                let mut previously_added_theses: BTreeMap<DocumentId, Thesis> = BTreeMap::new();
                for _ in 0..1000 {
                    let action_id = if previously_added_theses.is_empty() {
                        1
                    } else {
                        rng.generate_range(1..=3)
                    };
                    match action_id {
                        1 => {
                            let mut aliases_resolver = AliasesResolver {
                                read_able_transaction: transaction,
                                known_aliases: BTreeMap::new(),
                            };
                            let thesis = {
                                let mut result = random_thesis(
                                    &mut rng,
                                    &mut aliases_resolver,
                                    &previously_added_theses,
                                    &transaction,
                                );
                                while previously_added_theses.contains_key(&result.id()?) {
                                    result = random_thesis(
                                        &mut rng,
                                        &mut aliases_resolver,
                                        &previously_added_theses,
                                        &transaction,
                                    );
                                }
                                result
                            };
                            thesis.validated()?;
                            println!("add {:?}", thesis);
                            transaction.insert_thesis(thesis.clone())?;
                            for take_tags_amount in 1..thesis.tags.len() {
                                let taken_tags = thesis.tags[..take_tags_amount].to_vec();
                                for thesis_with_such_tags_id in transaction
                                    .iter_theses_ids_by_tags(&taken_tags, &vec![], None)?
                                    .collect::<Vec<_>>()?
                                {
                                    let thesis_with_such_tags =
                                        transaction.get_thesis(&thesis_with_such_tags_id)?.unwrap();
                                    for tag in taken_tags.iter() {
                                        assert!(thesis_with_such_tags.tags.contains(&tag));
                                    }
                                }
                            }
                            let thesis_id = thesis.id()?;
                            assert_eq!(transaction.get_thesis(&thesis_id)?.unwrap(), thesis);
                            for referenced_thesis_id in thesis.references() {
                                let where_referenced =
                                    transaction.where_referenced(&referenced_thesis_id)?;
                                assert!(where_referenced.contains(&thesis_id));
                            }
                            previously_added_theses.insert(thesis_id.clone(), thesis);
                            assert_eq!(
                                &transaction.get_thesis(&thesis_id)?.unwrap(),
                                previously_added_theses.get(&thesis_id).unwrap()
                            );
                        }
                        2 => {
                            let thesis_to_tag_id = previously_added_theses
                                .keys()
                                .nth(rng.generate_range(0..previously_added_theses.len()))
                                .unwrap()
                                .clone();
                            let thesis_to_tag =
                                previously_added_theses.get(&thesis_to_tag_id).unwrap();
                            let tag_to_add = {
                                let mut result = random_tag(&mut rng);
                                while thesis_to_tag.tags.contains(&result) {
                                    result = random_tag(&mut rng);
                                }
                                result
                            };
                            println!("tag {:?} with {:?}", thesis_to_tag_id, tag_to_add);
                            transaction.add_tags(&thesis_to_tag_id, [tag_to_add.clone()].into())?;
                            assert!(transaction
                                .get_thesis(&thesis_to_tag_id)?
                                .unwrap()
                                .tags
                                .contains(&tag_to_add));
                            previously_added_theses
                                .get_mut(&thesis_to_tag_id)
                                .unwrap()
                                .tags
                                .push(tag_to_add);
                            assert_eq!(
                                &transaction.get_thesis(&thesis_to_tag_id)?.unwrap(),
                                previously_added_theses.get(&thesis_to_tag_id).unwrap()
                            );
                        }
                        3 => {
                            if let Some((thesis_to_untag_id, thesis_to_untag)) =
                                previously_added_theses
                                    .iter()
                                    .find(|(_, thesis)| !thesis.tags.is_empty())
                                    .map(|(id, thesis)| (id.clone(), thesis.clone()))
                            {
                                assert_eq!(
                                    transaction.get_thesis(&thesis_to_untag_id)?.unwrap(),
                                    thesis_to_untag
                                );
                                let tag_to_remove_index =
                                    rng.generate_range(0..thesis_to_untag.tags.len());
                                let tag_to_remove =
                                    thesis_to_untag.tags[tag_to_remove_index].clone();
                                println!("untag {:?} with {:?}", thesis_to_untag_id, tag_to_remove);
                                transaction.remove_tags(
                                    &thesis_to_untag_id,
                                    &[tag_to_remove.clone()].into(),
                                )?;
                                assert!(!transaction
                                    .get_thesis(&thesis_to_untag_id)?
                                    .unwrap()
                                    .tags
                                    .contains(&tag_to_remove));
                                previously_added_theses
                                    .get_mut(&thesis_to_untag_id)
                                    .unwrap()
                                    .tags
                                    .remove(tag_to_remove_index);
                                assert_eq!(
                                    &transaction.get_thesis(&thesis_to_untag_id)?.unwrap(),
                                    previously_added_theses.get(&thesis_to_untag_id).unwrap()
                                );
                            }
                        }
                        _ => {}
                    }
                }
                for (thesis_id, thesis) in previously_added_theses.iter() {
                    assert_eq!(transaction.get_thesis(thesis_id)?.unwrap(), *thesis);
                }
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn test_example() {
        let mut sweater = new_default_sweater("test_example");
        sweater
            .lock_all_and_write(|transaction| {
                let mut aliases_resolver = AliasesResolver {
                    read_able_transaction: transaction,
                    known_aliases: BTreeMap::new(),
                };
                let mut commands = vec![];
                for line in std::fs::read_to_string("src/example.txt")?.lines() {
                    commands.push(Command::parse(
                        line,
                        &mut aliases_resolver,
                        &transaction.sweater_config.supported_relations_kinds,
                    )?);
                }
                for command in commands.iter() {
                    command.execute(transaction)?;
                }
                serde_json::to_string(&commands)?;

                std::fs::write(
                    "/tmp/wool_example_graph.dot",
                    GraphGenerator::new(&GraphGeneratorConfig { wrap_width: 64 }, transaction)?
                        .collect::<Vec<_>>()?
                        .join(""),
                )?;

                Ok(())
            })
            .unwrap();
    }
}
