use std::collections::BTreeSet;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::chest::wool_chest::{Chest, ChestConfig};
use crate::read_transaction::ReadTransaction;
use crate::relation::RelationKind;
use crate::write_transaction::WriteTransaction;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SweaterConfig {
    pub chest: ChestConfig,
    pub supported_relations_kinds: BTreeSet<RelationKind>,
}

pub struct Sweater {
    pub chest: Chest,
    pub config: SweaterConfig,
}

impl Sweater {
    pub fn new(config: SweaterConfig) -> Result<Self> {
        Ok(Self {
            chest: Chest::new(config.chest.clone()).with_context(|| {
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
