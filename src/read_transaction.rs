use anyhow::{Error, Result};
use fallible_iterator::FallibleIterator;
use trove::{path_segments, DocumentId, IndexRecordType};

use crate::alias::Alias;
use crate::chest::wool_chest;
use crate::sweater::SweaterConfig;
use crate::thesis::Thesis;

pub struct ReadTransaction<'a> {
    pub chest_transaction: &'a wool_chest::ReadTransaction<'a>,
    pub sweater_config: &'a SweaterConfig,
}

#[macro_export]
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

        fn get_thesis_id_by_alias(&self, alias: &Alias) -> Result<Option<DocumentId>> {
            Ok(self
                .chest_transaction
                .theses_select(
                    &vec![(
                        IndexRecordType::Direct,
                        path_segments!("alias"),
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
                        IndexRecordType::Array,
                        path_segments!("content", "Text", "references"),
                        json_value.clone(),
                    )],
                    &vec![],
                    None,
                )?
                .chain(self.chest_transaction.theses_select(
                    &vec![(
                        IndexRecordType::Direct,
                        path_segments!("content", "Relation", "from"),
                        json_value.clone(),
                    )],
                    &vec![],
                    None,
                )?)
                .chain(self.chest_transaction.theses_select(
                    &vec![(
                        IndexRecordType::Direct,
                        path_segments!("content", "Relation", "to"),
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
    fn get_thesis_id_by_alias(&self, alias: &Alias) -> Result<Option<DocumentId>>;
    fn get_alias_by_thesis_id(&self, thesis_id: &DocumentId) -> Result<Option<Alias>>;
    fn where_referenced(&self, thesis_id: &DocumentId) -> Result<Vec<DocumentId>>;
    fn iter_theses(&self) -> Result<Box<dyn FallibleIterator<Item = Thesis, Error = Error> + '_>>;
}

impl<'a> ReadTransactionMethods<'a> for ReadTransaction<'a> {
    define_read_methods!('a);
}
