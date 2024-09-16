use chrono::{DateTime, Utc};
use crate::U256;
use uuid::Uuid;

pub struct Blockchain {
    pub blocks: Vec<Block>,
}
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}
pub struct Transaction {
    pub inputs: Vec<TransactionInput>,
    pub output: Vec<TransactionOutput>,
}
impl Blockchain {
    pub fn new() -> Self {
        Blockchain { blocks: vec![] }
    }
    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }
}
pub struct BlockHeader {
    // when the block was created
    pub timestamp: DateTime<Utc>,
    // only used once, we increment it to mine the block
    pub nonce: u64,
    // hash of the previous block
    pub prev_block_hash: [u8; 32],
    // Merkle root of the blocks transactions. Ensures all transactions are accounted for and unalterable without changing the header
    pub merkle_root: [u8; 32],
    // a number which has to be higher than the hash of this block for it to be considered valid
    pub target: U256,
}
impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Block {
            header,
            transactions,
        }
    }
    pub fn hash(&self) -> ! {
        unimplemented!()
    }
}
impl BlockHeader {
    pub fn new(
        timestamp: DateTime<Utc>,
        nonce: u64,
        prev_block_hash: [u8; 32],
        merkle_root: [u8; 32],
        target: U256,
    ) -> Self {
        BlockHeader {
            timestamp,
            nonce,
            prev_block_hash,
            merkle_root,
            target,
        }
    }
    pub fn hash(&self) -> ! {
        unimplemented!()
    }
}
impl Transaction {
    pub fn new(inputs: Vec<TransactionInput>, output: Vec<TransactionOutput>) -> Self {
        Transaction { inputs, output }
    }
    pub fn hash(&self) -> ! {
        unimplemented!()
    }
}
pub struct TransactionInput {
    pub prev_transaction_output_hash: [u8; 32],
    pub signature: [u8; 32], // dummy types will be replaced later
}
pub struct TransactionOutput {
    pub value: u64,
    pub unique_id: Uuid,
    pub pubkey: [u8; 32], // dummy types will be replaced later
}
