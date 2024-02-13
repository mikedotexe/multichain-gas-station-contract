use std::str::FromStr;

use ethers_core::types::U256;
use near_sdk::{
    env,
    json_types::{U128, U64},
    near_bindgen, require,
    store::Vector,
    AccountId, Promise, PromiseError,
};
use near_sdk_contract_tools::owner::{Owner, OwnerExternal};

use crate::{
    asset::AssetId,
    chain_configuration::{ChainConfiguration, PaymasterConfiguration},
    decode_transaction_request,
    foreign_address::ForeignAddress,
    kdf::get_mpc_address,
    oracle::PriceData,
    signer_contract::ext_signer,
    valid_transaction_request::ValidTransactionRequest,
    Contract, ContractExt, Flags, GetForeignChain, PendingTransactionSequence, StorageKey,
};

#[allow(clippy::needless_pass_by_value)]
#[near_bindgen]
impl Contract {
    pub fn get_expire_sequence_after_ns(&self) -> U64 {
        self.expire_sequence_after_ns.into()
    }

    pub fn set_expire_sequence_after_ns(&mut self, expire_sequence_after_ns: U64) {
        self.assert_owner();
        self.expire_sequence_after_ns = expire_sequence_after_ns.into();
    }

    pub fn get_signer_contract_id(&self) -> &AccountId {
        &self.signer_contract_id
    }

    /// Set the signer contract ID.
    /// Requires a call to [`Contract::refresh_signer_public_key`] afterwards.
    pub fn set_signer_contract_id(&mut self, account_id: AccountId) {
        self.assert_owner();
        if self.signer_contract_id != account_id {
            self.signer_contract_id = account_id;
            self.signer_contract_public_key = None;
        }
    }

    /// Refresh the public key from the signer contract.
    pub fn refresh_signer_public_key(&mut self) -> Promise {
        self.assert_owner();

        ext_signer::ext(self.signer_contract_id.clone())
            .public_key()
            .then(Self::ext(env::current_account_id()).refresh_signer_public_key_callback())
    }

    #[private]
    pub fn refresh_signer_public_key_callback(
        &mut self,
        #[callback_result] public_key: Result<near_sdk::PublicKey, PromiseError>,
    ) {
        let public_key = public_key.unwrap_or_else(|_| {
            env::panic_str("Failed to load signer public key from the signer contract")
        });
        self.signer_contract_public_key = Some(public_key);
    }

    pub fn get_flags(&self) -> &Flags {
        &self.flags
    }

    pub fn set_flags(&mut self, flags: Flags) {
        self.assert_owner();
        self.flags = flags;
    }

    pub fn get_receiver_whitelist(&self) -> Vec<&ForeignAddress> {
        self.receiver_whitelist.iter().collect()
    }

    pub fn add_to_receiver_whitelist(&mut self, addresses: Vec<ForeignAddress>) {
        self.assert_owner();
        for address in addresses {
            self.receiver_whitelist.insert(address);
        }
    }

    pub fn remove_from_receiver_whitelist(&mut self, addresses: Vec<ForeignAddress>) {
        self.assert_owner();
        for address in addresses {
            self.receiver_whitelist.remove(&address);
        }
    }

    pub fn clear_receiver_whitelist(&mut self) {
        self.assert_owner();
        self.receiver_whitelist.clear();
    }

    pub fn get_sender_whitelist(&self) -> Vec<&AccountId> {
        self.sender_whitelist.iter().collect()
    }

    pub fn add_to_sender_whitelist(&mut self, addresses: Vec<AccountId>) {
        self.assert_owner();
        for address in addresses {
            self.sender_whitelist.insert(address);
        }
    }

    pub fn remove_from_sender_whitelist(&mut self, addresses: Vec<AccountId>) {
        self.assert_owner();
        for address in addresses {
            self.sender_whitelist.remove(&address);
        }
    }

    pub fn clear_sender_whitelist(&mut self) {
        self.assert_owner();
        self.sender_whitelist.clear();
    }

    pub fn add_foreign_chain(
        &mut self,
        chain_id: U64,
        oracle_asset_id: String,
        transfer_gas: U128,
        fee_rate: (U128, U128),
    ) {
        self.assert_owner();

        self.foreign_chains.insert(
            chain_id.0,
            ChainConfiguration {
                next_paymaster: 0,
                oracle_asset_id,
                transfer_gas: U256::from(transfer_gas.0).0,
                fee_rate: (fee_rate.0.into(), fee_rate.1.into()),
                paymasters: Vector::new(StorageKey::Paymasters(chain_id.0)),
            },
        );
    }

    pub fn set_foreign_chain_oracle_asset_id(&mut self, chain_id: U64, oracle_asset_id: String) {
        self.assert_owner();
        if let Some(config) = self.foreign_chains.get_mut(&chain_id.0) {
            config.oracle_asset_id = oracle_asset_id;
        } else {
            env::panic_str("Foreign chain does not exist");
        }
    }

    pub fn set_foreign_chain_transfer_gas(&mut self, chain_id: U64, transfer_gas: U128) {
        self.assert_owner();
        if let Some(config) = self.foreign_chains.get_mut(&chain_id.0) {
            config.transfer_gas = U256::from(transfer_gas.0).0;
        } else {
            env::panic_str("Foreign chain does not exist");
        }
    }

    pub fn remove_foreign_chain(&mut self, chain_id: U64) {
        self.assert_owner();
        if let Some((_, mut config)) = self.foreign_chains.remove_entry(&chain_id.0) {
            config.paymasters.clear();
        }
    }

    pub fn get_foreign_chains(&self) -> Vec<GetForeignChain> {
        self.foreign_chains
            .iter()
            .map(|(chain_id, config)| GetForeignChain {
                chain_id: (*chain_id).into(),
                oracle_asset_id: config.oracle_asset_id.clone(),
            })
            .collect()
    }

    pub fn add_paymaster(&mut self, chain_id: U64, nonce: u32, key_path: String) -> u32 {
        self.assert_owner();

        require!(
            AccountId::from_str(&key_path).is_err(),
            "Paymaster key path must not be a valid account id",
        );

        let chain = self
            .foreign_chains
            .get_mut(&chain_id.0)
            .unwrap_or_else(|| env::panic_str("Foreign chain does not exist"));

        let index = chain.paymasters.len();

        chain
            .paymasters
            .push(PaymasterConfiguration { nonce, key_path });

        index
    }

    pub fn set_paymaster_nonce(&mut self, chain_id: U64, index: u32, nonce: u32) {
        self.assert_owner();
        let chain = self
            .foreign_chains
            .get_mut(&chain_id.0)
            .unwrap_or_else(|| env::panic_str("Foreign chain does not exist"));

        let paymaster = chain.paymasters.get_mut(index).unwrap_or_else(|| {
            env::panic_str("Invalid index");
        });

        paymaster.nonce = nonce;
    }

    /// Note: If a transaction is _already_ pending signatures with the
    /// paymaster getting removed, this method will not prevent those payloads
    /// from getting signed.
    pub fn remove_paymaster(&mut self, chain_id: U64, index: u32) {
        self.assert_owner();
        let chain = self
            .foreign_chains
            .get_mut(&chain_id.0)
            .unwrap_or_else(|| env::panic_str("Foreign chain does not exist"));

        if index < chain.paymasters.len() {
            chain.paymasters.swap_remove(index);
            // resetting chain.next_paymaster is not necessary, since overflow is handled in [`ForeignChainConfiguration::next_paymaster`] function.
        } else {
            env::panic_str("Invalid index");
        }
    }

    pub fn get_paymasters(&self, chain_id: U64) -> Vec<&PaymasterConfiguration> {
        self.foreign_chains
            .get(&chain_id.0)
            .unwrap_or_else(|| env::panic_str("Foreign chain does not exist"))
            .paymasters
            .iter()
            .collect()
    }

    pub fn list_transactions(
        &self,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> std::collections::HashMap<String, &PendingTransactionSequence> {
        let mut v: Vec<_> = self.pending_transaction_sequences.iter().collect();

        v.sort_by_cached_key(|&(id, _)| *id);

        v.into_iter()
            .skip(offset.map_or(0, |o| o as usize))
            .take(limit.map_or(usize::MAX, |l| l as usize))
            .map(|(id, tx)| (id.to_string(), tx))
            .collect()
    }

    pub fn get_transaction(&self, id: U64) -> Option<&PendingTransactionSequence> {
        self.pending_transaction_sequences.get(&id.0)
    }

    pub fn withdraw_collected_fees(
        &mut self,
        asset_id: AssetId,
        amount: Option<U128>,
        receiver_id: Option<AccountId>, // TODO: Pull method instead of push (danger of typos/locked accounts)
    ) -> Promise {
        near_sdk::assert_one_yocto();
        self.assert_owner();
        let fees = self
            .collected_fees
            .get_mut(&asset_id)
            .unwrap_or_else(|| env::panic_str("No fee entry for provided asset ID"));

        let amount = amount.unwrap_or(U128(fees.0));

        fees.0 = fees
            .0
            .checked_sub(amount.0)
            .unwrap_or_else(|| env::panic_str("Not enough fees to withdraw"));

        asset_id.transfer(
            receiver_id.unwrap_or_else(|| self.own_get_owner().unwrap()),
            amount,
        )
    }

    pub fn get_collected_fees(&self) -> std::collections::HashMap<&AssetId, &U128> {
        self.collected_fees.iter().collect()
    }

    pub fn get_foreign_address_for(&self, account_id: AccountId) -> ForeignAddress {
        get_mpc_address(
            self.signer_contract_public_key.clone().unwrap(),
            &env::current_account_id(),
            account_id.as_str(),
        )
        .unwrap()
    }

    pub fn estimate_gas_cost(&self, transaction_rlp_hex: String, price_data: PriceData) -> U128 {
        let transaction =
            ValidTransactionRequest::try_from(decode_transaction_request(&transaction_rlp_hex))
                .unwrap_or_else(|e| env::panic_str(&format!("Invalid transaction request: {e}")));

        let foreign_chain_configuration = self
            .foreign_chains
            .get(&transaction.chain_id)
            .unwrap_or_else(|| {
                env::panic_str(&format!(
                    "Paymaster not supported for chain id {}",
                    transaction.chain_id
                ))
            });

        let paymaster_transaction_gas = foreign_chain_configuration.transfer_gas();
        let request_tokens_for_gas =
            (transaction.gas() + paymaster_transaction_gas) * transaction.gas_price();

        foreign_chain_configuration
            .foreign_token_price(
                &self.oracle_local_asset_id,
                &price_data,
                request_tokens_for_gas,
            )
            .into()
    }
}