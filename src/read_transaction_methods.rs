use std::collections::BTreeSet;

use anyhow::{Error, Result};
use fallible_iterator::FallibleIterator;
use trove::DocumentId;

use crate::alias::Alias;
use crate::relation_kind::RelationKind;
use crate::tag::Tag;
use crate::text::Text;
use crate::thesis::Thesis;

pub trait ReadTransactionMethods<'a> {
    fn get_thesis(&self, thesis_id: &DocumentId) -> Result<Option<Thesis>>;
    fn iter_theses_ids_by_tags(
        &self,
        present_tags: &Vec<Tag>,
        absent_tags: &Vec<Tag>,
        start_after_thesis_id: Option<DocumentId>,
    ) -> Result<Box<dyn FallibleIterator<Item = DocumentId, Error = Error> + '_>>;
    fn get_thesis_id_by_alias(&self, alias: &Alias) -> Result<Option<DocumentId>>;
    fn get_alias_by_thesis_id(&self, thesis_id: &DocumentId) -> Result<Option<Alias>>;
    fn where_referenced(&self, thesis_id: &DocumentId) -> Result<Vec<DocumentId>>;
    fn iter_theses(&self) -> Result<Box<dyn FallibleIterator<Item = Thesis, Error = Error> + '_>>;
    fn compose_with_aliases(&self, text: &Text) -> Result<String>;
    fn supported_relations_kinds(&self) -> BTreeSet<RelationKind>;
}
