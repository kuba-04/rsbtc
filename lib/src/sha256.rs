use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use sha256::digest;

use crate::U256;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Hash(U256);
impl Hash {
    pub fn hash<T: serde::Serialize>(data: &T) -> Self {
        let mut serialized: Vec<u8> = vec![];

        if let Err(e) =
            ciborium::into_writer(data, &mut serialized)
        {
            panic!(
                "Failed to serialize data: {:?}. \
                This should not happen",
                e
            );
        }
        let hash = digest(&serialized);
        let hash_bytes = hex::decode(hash).unwrap();
        let hash_array: [u8; 32] = hash_bytes.as_slice()
            .try_into()
            .unwrap();
        Hash(U256::from(hash_array))
    }
    // check if hash matches the target
    pub fn matches_target(&self, target: U256) -> bool {
        self.0 <= target
    }
    // zero hash
    pub fn zero() -> Self {
        Hash(U256::zero())
    }
    pub fn as_bytes(&self) -> [u8; 32] {
        let mut bytes: Vec<u8> = vec![0; 32];
        self.0.to_little_endian(&mut bytes);
        bytes.as_slice().try_into().unwrap()
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn hash_test() {
        // let input_data = "Hello world".to_string();
        // let output_hash = Hash::hash(&input_data);

        // asser
        // let input = "hello";
        // let val = digest(input);
        // assert_eq!(val,"2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
    }
}
