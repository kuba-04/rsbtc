use k256::elliptic_curve::consts::U2;
use crate::U256;
use sha256::digest;
use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub struct Hash(U256);

impl Hash {
    pub fn hash<T: serde::Serialize>(data: &T) -> Self {
        let mut serialized: Vec<u8> = vec![];
        if let Err(e) = ciborium::into_writer(
            data,
            &mut serialized,
        ) {
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
}

#[cfg(test)]
mod test {

    use crate::sha256::Hash;
    use sha256::digest;
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
