pub mod alias;
pub mod aliases_resolver;
pub mod command;
pub mod content;
pub mod graph_generator;
pub mod read_transaction_methods;
pub mod reference;
pub mod relation;
pub mod relation_kind;
pub mod tag;
pub mod text;
pub mod thesis;

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
                    text::{Text, RawText}, alias::Alias, reference::Reference, relation::Relation,
                    relation_kind::RelationKind, tag::Tag, content::Content, thesis::Thesis,
                    aliases_resolver::AliasesResolver, command::Command, read_transaction_methods::ReadTransactionMethods,
                    trove::{define_chest, path_segments, search_path_segments, DocumentId},
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

                    fn compose_with_aliases(
                        &self,
                        text: &Text
                    ) -> Result<String> {
                        text.composed(|reference|
                                    Ok(if let Some(alias) = self.get_alias_by_thesis_id(reference)? {
                                        alias.0
                                    } else {
                                        reference.to_string()
                                    }))
                    }

                    fn supported_relations_kinds(&self) -> BTreeSet<RelationKind> {
                        self.sweater_config.supported_relations_kinds.clone()
                    }

                    fn backup_to_commands(&self) -> Result<Vec<Command>> {
                        let mut result = vec![];
                        let mut already_got_theses_ids: BTreeSet<DocumentId> = BTreeSet::new();
                        let mut stack: Vec<(Thesis, Vec<DocumentId>)> = vec![];
                        let mut all_theses_iterator = self.iter_theses()?;
                        while true {
                            if let Some(thesis_and_references) = stack.last_mut() {
                                if let Some(ref reference) = thesis_and_references.1.pop() {
                                    let new_thesis = self.get_thesis(reference)?.unwrap();
                                    let new_thesis_id = new_thesis.id();
                                    if !already_got_theses_ids.contains(&new_thesis_id) {
                                        already_got_theses_ids.insert(new_thesis_id);
                                        let new_thesis_references = new_thesis.references();
                                        stack.push((new_thesis, new_thesis_references));
                                    }
                                } else {
                                    result.extend(thesis_and_references.0.to_commands());
                                    stack.pop();
                                }
                            } else {
                                if let Some(thesis) = all_theses_iterator.next()? {
                                    let thesis_references = thesis.references();
                                    stack.push((thesis, thesis_references));
                                } else {
                                    break;
                                }
                            }
                        }
                        Ok(result)
                    }
                };
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
                    let thesis_id = thesis.id();
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

                pub fn execute_command(&mut self, command: &Command) -> Result<()> {
                    match command {
                        Command::AddTextThesisWithAlias { text, alias } => self.insert_thesis(Thesis {
                            alias: Some(alias.clone()),
                            content: Content::Text(text.clone()),
                            tags: vec![]
                        }),
                        Command::AddTextThesisWithoutAlias(text) => self.insert_thesis(Thesis {
                            alias: None,
                            content: Content::Text(text.clone()),
                            tags: vec![]
                        }),
                        Command::AddRelationThesisWithAlias { relation, alias } => {
                            self.insert_thesis(Thesis {
                                alias: Some(alias.clone()),
                                content: Content::Relation(relation.clone()),
                                tags: vec![]
                            })
                        }
                        Command::AddRelationThesisWithoutAlias(relation) => {
                            self.insert_thesis(Thesis {
                                alias: None,
                                content: Content::Relation(relation.clone()),
                                tags: vec![]
                            })
                        }
                        Command::SetAlias { thesis_id, alias } => {
                            self.set_alias(thesis_id.clone(), alias.clone())
                        }
                        Command::AddTags { thesis_id, tags } => self.add_tags(thesis_id, tags.clone()),
                        Command::RemoveTags { thesis_id, tags } => self.remove_tags(thesis_id, &tags),
                    }
                }
            }

            pub struct LocalAliasesResolver<'a> {
                pub read_able_transaction: &'a dyn ReadTransactionMethods<'a>,
                pub known_aliases: BTreeMap<Alias, DocumentId>,
            }

            impl<'a> AliasesResolver for LocalAliasesResolver<'a> {
                fn get_thesis_id_by_reference(&self, reference: &Reference) -> Result<DocumentId> {
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

                fn remember(&mut self, alias: Alias, document_id: DocumentId) {
                    self.known_aliases.insert(alias, document_id);
                }

                fn new_text(&self, input: &str) -> Result<Text> {
                    static REFERENCE_IN_TEXT_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
                    let reference_in_text_regex = REFERENCE_IN_TEXT_REGEX.get_or_init(|| {
                        Regex::new(r#"\[(:?([A-Za-z0-9-_]{22})|([^\[\]]+))\]"#)
                            .with_context(|| {
                                "Can not compile regular expression to split text on raw text parts and \
                                 references"
                            })
                            .unwrap()
                    });

                    let mut result = Text {
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
                                self
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
        crate::{
            aliases_resolver::AliasesResolver,
            command::Command,
            content::Content,
            graph_generator::{GraphGenerator, GraphGeneratorConfig},
            read_transaction_methods::ReadTransactionMethods,
            relation::Relation,
            tag::Tag,
            text::Text,
            thesis::Thesis,
        },
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
        transaction: &WriteTransaction,
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
        let aliases_resolver = Box::new(LocalAliasesResolver {
            read_able_transaction: transaction,
            known_aliases: BTreeMap::new(),
        }) as Box<dyn AliasesResolver>;
        let result = aliases_resolver.new_text(&result_string).unwrap();
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
                    1 => Content::Text(random_text(rng, previously_added_theses, transaction)),
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
                            let thesis = {
                                let mut result =
                                    random_thesis(&mut rng, &previously_added_theses, &transaction);
                                while previously_added_theses.contains_key(&result.id()) {
                                    result = random_thesis(
                                        &mut rng,
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
                            let thesis_id = thesis.id();
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
                let commands = {
                    let mut aliases_resolver = LocalAliasesResolver {
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
                    commands
                };
                serde_json::to_string(&commands)?;
                for command in commands.iter() {
                    transaction.execute_command(command)?;
                }

                std::fs::write(
                    "/tmp/wool_example_graph.dot",
                    GraphGenerator::new(&GraphGeneratorConfig { wrap_width: 64 }, transaction)?
                        .collect::<Vec<_>>()?
                        .join(""),
                )?;

                for command in transaction.backup_to_commands()? {
                    match command {
                        Command::AddTextThesisWithAlias { text, alias } => {
                            let content = Content::Text(text);
                            let thesis_in_sweater = transaction.get_thesis(&content.id())?.unwrap();
                            assert_eq!(content, thesis_in_sweater.content);
                            assert_eq!(Some(alias), thesis_in_sweater.alias);
                        }
                        Command::AddTextThesisWithoutAlias(text) => {
                            let content = Content::Text(text);
                            let thesis_in_sweater = transaction.get_thesis(&content.id())?.unwrap();
                            assert_eq!(content, thesis_in_sweater.content);
                        }
                        Command::AddRelationThesisWithAlias { relation, alias } => {
                            let content = Content::Relation(relation);
                            let thesis_in_sweater = transaction.get_thesis(&content.id())?.unwrap();
                            assert_eq!(content, thesis_in_sweater.content);
                            assert_eq!(Some(alias), thesis_in_sweater.alias);
                        }
                        Command::AddRelationThesisWithoutAlias(relation) => {
                            let content = Content::Relation(relation);
                            let thesis_in_sweater = transaction.get_thesis(&content.id())?.unwrap();
                            assert_eq!(content, thesis_in_sweater.content);
                        }
                        _ => {}
                    }
                }

                Ok(())
            })
            .unwrap();
    }
}
