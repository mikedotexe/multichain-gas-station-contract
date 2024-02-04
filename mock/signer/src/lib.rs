use near_multichain_gas_station_contract::signer_contract::{
    MpcSignature, ProtocolContractState, SignerContract,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen, PromiseOrValue, PublicKey,
};

#[derive(BorshSerialize, BorshDeserialize, Default, Debug)]
#[near_bindgen]
struct Contract {}

#[near_bindgen]
impl SignerContract for Contract {
    fn sign(&mut self, payload: [u8; 32], path: &String) -> PromiseOrValue<MpcSignature> {
        let _ = (payload, path);
        PromiseOrValue::Value(MpcSignature::new([0; 32], [0; 32], 0))
    }

    fn state(&self) -> ProtocolContractState {
        todo!()
    }

    fn public_key(&self) -> PublicKey {
        "secp256k1:qMoRgcoXai4mBPsdbHi1wfyxF9TdbPCF4qSDQTRP3TfescSRoUdSx6nmeQoN3aiwGzwMyGXAb1gUjBTv5AY8DXj"
             .parse()
             .unwrap()
    }
}