use std::collections::{HashMap, HashSet};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::error::BtcError;
use crate::sha256::Hash;
use crate::types::block::Block;
use crate::types::transaction::{Transaction, TransactionOutput};
use crate::U256;
use crate::util::{MerkleRoot, Saveable};
use std::io::{
    Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult, Write
};

// #[derive(Serialize, Deserialize, Clone, Debug)]
#[derive(Serialize, Deserialize)]
pub struct Blockchain {
    utxos: HashMap<Hash, (bool, TransactionOutput)>,
    target: U256,
    blocks: Vec<Block>,
    #[serde(default, skip_serializing)]
    mempool: Vec<(DateTime<Utc>, Transaction)>,
}
impl Blockchain {
    pub fn new() -> Self {
        Blockchain {
            utxos: HashMap::new(),
            target: crate::MIN_TARGET,
            blocks: vec![],
            mempool: Vec::<(DateTime<Utc>, Transaction)>::new(),
        }
    }
    pub fn utxos(&self) -> &HashMap<Hash, (bool, TransactionOutput)> {
        &self.utxos
    }
    pub fn target(&self) -> U256 {
        self.target
    }
    pub fn blocks(&self) -> impl Iterator <Item = &Block> {
        self.blocks.iter()
    }
    pub fn block_height(&self) -> u64 {
        self.blocks.len() as u64
    }
    pub fn mempool(&self) -> &Vec<(DateTime<Utc>, Transaction)> {
        &self.mempool
    }
    pub fn add_block(&mut self, block: Block) -> crate::error::Result<()> {
        if self.blocks.is_empty() {
            // if first block, check if prev block hash is all zeros
            if block.header.prev_block_hash != Hash::zero() {
                println!("zero hash");
                return Err(BtcError::InvalidBlock);
            }
        } else {
            // if not the first block, check if prev block hash is the hash of the last block
            let last_block = self.blocks.last().unwrap();
            if block.header.prev_block_hash != last_block.hash() {
                println!("prev hash is wrong");
                return Err(BtcError::InvalidBlock);
            }
            // check if the blocks hash is less than the target
            if !block.header.hash().matches_target(block.header.target) {
                println!("does not match target");
                return Err(BtcError::InvalidBlock);
            }
            // check if the block's merkle root is correct
            let calculated_merkle_root = MerkleRoot::calculate(&block.transactions);
            if calculated_merkle_root != block.header.merkle_root {
                println!("invalid merkle root");
                return Err(BtcError::InvalidMerkleRoot);
            }
            // check if the block's timestamp is after the last blocks's timestamp
            if block.header.timestamp <= last_block.header.timestamp {
                println!("invalid timestamp");
                return Err(BtcError::InvalidBlock);
            }
            // verify all tx in the block
            block.verify_transactions(
                self.block_height(),
                &self.utxos,
            )?;
        }
        // Remove transactions from mempool that are now in the block
        let block_transactions: HashSet<_> = block
            .transactions
            .iter()
            .map(|tx| tx.hash())
            .collect();
        self.mempool.retain(|(_, tx)| {
            !block_transactions.contains(&tx.hash())
        });
        self.blocks.push(block);
        self.try_adjust_target();
        Ok(())
    }
    pub fn add_to_mempool(&mut self, transaction: Transaction) -> crate::error::Result<()> {
        // validate transaction before insertion
        // all inputs must match known UTXOs, and must be unique
        let mut known_inputs = HashSet::new();
        for input in &transaction.inputs {
            if !self.utxos.contains_key(&input.prev_transaction_output_hash) {
                return Err(BtcError::InvalidTransaction);
            }
            if known_inputs.contains(&input.prev_transaction_output_hash) {
                return Err(BtcError::InvalidTransaction);
            }
            known_inputs.insert(input.prev_transaction_output_hash);
        }
        // check if any of the utxos have the bool mark set to true
        // and if so, find the transaction that references them in mempool, remove it,
        // and set all the utxos it references to false
        for input in &transaction.inputs {
            if let Some((true, _)) = self
                .utxos
                .get(&input.prev_transaction_output_hash) {
                // find the transaction that references the UTXO we are trying to reference
                let referencing_transaction = self.mempool
                    .iter()
                    .enumerate()
                    .find(|(_, (_, transaction))|
                    transaction.outputs.iter().any(|output| output.hash() == input.prev_transaction_output_hash));
                // if we have found one, unmark all of its UTXOs
                if let Some((idx, (_, referencing_transaction))) = referencing_transaction {
                    for input in &referencing_transaction.inputs {
                        // set all utxos from this tx to false
                        self.utxos
                            .entry(input.prev_transaction_output_hash)
                            .and_modify(|(marked, _)| {
                                *marked = false;
                            });
                    }
                    // remove the tx from the mempool
                    self.mempool.remove(idx);
                } else {
                    // if somehow there is no matching tx, set this utxo to false
                    self.utxos
                        .entry(input.prev_transaction_output_hash)
                        .and_modify(|(marked, _)| {
                            *marked = false;
                        });
                }
            }
        }

        // all inputs must be lower than all outputs
        let all_inputs = transaction
            .inputs
            .iter()
            .map(|input| {
                self.utxos
                    .get(&input.prev_transaction_output_hash)
                    .expect("BUG: impossible")
                    .1
                    .value
            })
            .sum::<u64>();
        let all_outputs: u64 = transaction
            .outputs
            .iter()
            .map(|output| output.value)
            .sum();
        if all_inputs < all_outputs {
            return Err(BtcError::InvalidTransaction);
        }
        // push the transaction to the mempool
        self.mempool.push((Utc::now(), transaction));
        // sort by miner fee
        self.mempool.sort_by_key(|(_, transaction)| {
            let all_inputs = transaction
                .inputs
                .iter()
                .map(|input| {
                    self.utxos
                        .get(&input.prev_transaction_output_hash)
                        .expect("BUG: impossible")
                        .1
                        .value
                })
                .sum::<u64>();
            let all_outputs: u64 = transaction
                .outputs
                .iter()
                .map(|output| output.value)
                .sum();
            // Mark the UTXOs as used
            for input in &transaction.inputs {
                self.utxos
                    .entry(input.prev_transaction_output_hash)
                    .and_modify(|(marked, _)| {
                        *marked = true;
                    });
            }
            let miner_fee = all_inputs - all_outputs;
            miner_fee
        });
        Ok(())
    }
    pub fn try_adjust_target(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        if self.blocks.len() % crate::DIFFICULTY_UPDATE_INTERVAL as usize != 0 {
            return;
        }
        // measure the time it took to mine the last DIFFICULTY_UPDATE_INTERVAL blocks with chrono
        let start_time = self.blocks[self.blocks.len() - crate::DIFFICULTY_UPDATE_INTERVAL as usize]
            .header
            .timestamp;
        let end_time = self.blocks.last().unwrap().header.timestamp;
        let time_diff = end_time - start_time;
        let time_diff_seconds = time_diff.num_seconds();
        let target_seconds = crate::IDEAL_BLOCK_TIME * crate::DIFFICULTY_UPDATE_INTERVAL;
        // mult the current target by actual time divided by ideal time
        let new_target = BigDecimal::parse_bytes(&self.target.to_string().as_bytes(), 10).expect("BUG: impossible")
            * BigDecimal::from(time_diff_seconds)
            / BigDecimal::from(target_seconds);
        let new_target_str = new_target.to_string()
            .split(".")
            .next()
            .expect("BUG: Expected a decimal point")
            .to_owned();
        let new_target: U256 = U256::from_str_radix(&new_target_str, 10)
            .expect("BUG: impossible");
        // clamp new target to be within the range of 4 * self.target and self.target / 4
        let new_target = if new_target < self.target / 4 {
            self.target / 4
        } else if new_target > self.target * 4 {
            self.target * 4
        } else {
            new_target
        };
        // if new target is more than the minimum target, set it to the min target
        self.target = new_target.min(crate::MIN_TARGET);
    }
    pub fn rebuild_utxos(&mut self) {
        for block in &self.blocks {
            for transaction in &block.transactions {
                for input in &transaction.inputs {
                    self.utxos.remove(
                        &input.prev_transaction_output_hash,
                    );
                }
                for output in transaction.outputs.iter() {
                    self.utxos.insert(
                        transaction.hash(),
                        (false, output.clone()),
                    );
                }
            }
        }
    }
    // remove transactions older than MAX_MEMPOOL_TRANSACTION_AGE
    pub fn cleanup_mempool(&mut self) {
        let now = Utc::now();
        let mut utxo_hashes_to_unmark: Vec<Hash> = vec![];
        self.mempool.retain(|(timestamp, transaction)| {
            if now - *timestamp > chrono::Duration::seconds(crate::MAX_MEMPOOL_TRANSACTION_AGE as i64) {
                // push all utxos to unmark to the vector so we can unmarrk them later
                utxo_hashes_to_unmark
                    .extend(transaction.inputs.iter().map(|input| input.prev_transaction_output_hash));
                false
            } else {
                true
            }
        });
        // unmark all of the UTXOs
        for hash in utxo_hashes_to_unmark {
            self.utxos.entry(hash).and_modify(|(marked, _)| *marked = false);
        }
    }
}
impl Saveable for Blockchain {
    fn load<I: Read>(reader: I) -> IoResult<Self> {
        ciborium::de::from_reader(reader).map_err(|_| {
            IoError::new(
                IoErrorKind::InvalidData,
                "Failed to deserialize Blockchain",
            )
        })
    }

    fn save<O: Write>(&self, writer: O) -> IoResult<()> {
        ciborium::ser::into_writer(self, writer).map_err(
            |_| {
                IoError::new(
                    IoErrorKind::InvalidData,
                    "Failed to serialize Blockchain",
                )
            }
        )
    }
}