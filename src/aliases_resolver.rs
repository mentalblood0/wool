use anyhow::Result;
use trove::DocumentId;

use crate::alias::Alias;
use crate::reference::Reference;
use crate::text::Text;

pub trait AliasesResolver {
    fn get_thesis_id_by_reference(&self, reference: &Reference) -> Result<DocumentId>;
    fn remember(&mut self, alias: Alias, document_id: DocumentId);
    fn new_text(&self, input: &str) -> Result<Text>;
}
