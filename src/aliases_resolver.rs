use anyhow::{anyhow, Result};
use std::collections::BTreeMap;
use trove::DocumentId;

use crate::{alias::Alias, commands::Reference, read_transaction::ReadTransactionMethods};

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
