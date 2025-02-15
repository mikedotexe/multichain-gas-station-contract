# NFT Chain Keys

The MPC contract provides a `sign` function that accepts a `path` parameter. This allows one predecessor account to have access to an effectively unlimited number of MPC keys.

The NFT chain key contract in this directory takes advantage of that property to allow MPC keys to be transferred between users, securely, using [the NEP-171 NFT contract standard](https://nomicon.io/Standards/Tokens/NonFungibleToken/Core).

## Usage

This contract conforms to a full suite of contract standards:

- [NEP-145: Storage Management](https://nomicon.io/Standards/StorageManagement)
- [NEP-171: NFT Core](https://nomicon.io/Standards/Tokens/NonFungibleToken/Core)
- [NEP-177: Metadata](https://nomicon.io/Standards/Tokens/NonFungibleToken/Metadata)
- [NEP-178: Approval Management](https://nomicon.io/Standards/Tokens/NonFungibleToken/ApprovalManagement)
- [NEP-181: Enumeration](https://nomicon.io/Standards/Tokens/NonFungibleToken/Enumeration)

Please refer to the corresponding documentation for usage information.

This contract also implements new functionality to enable the aforementioned chain key management features.

### Creating new key tokens

After [registering for storage](https://nomicon.io/Standards/StorageManagement), an account can mint unlimited new NFT chain keys using the `mint` method, as long as they have sufficient storage. The method is unlimited because it doesn't really cost very much to create a new NFT chain key&mdash;the value comes in what the key is used to do. The `mint` function does not accept any arguments, and returns the ID of the newly minted token.

### Issuing signatures

New signatures can be generated by calling `ckt_sign_hash` with a payload like so:

```json
{
  "token_id": "0",
  "payload": [
    156, 8, 177, 158, 80, 176, 41, 184, 237, 165, 187, 240, 235, 145, 121, 244,
    65, 29, 44, 161, 51, 56, 243, 238, 255, 78, 255, 22, 40, 71, 246, 81
  ]
}
```

The method will return the signature as a hexadecimal string:

```json
"ea58c007578b16f21ff28fd2aae22e7fd30376560f848582a32bca913bfced1d55eab4c3248efd0fa1a989a8a69a1841ac9e292c03125a20622fbe4da42ded5600"
```

### Approvals

While there already exists an approvals standard for _transferring_ NFTs, there does not exist an approvals standard for _using_ NFTs, which is an intrinsically different operation.

Therefore, this contract implements a separate set of approval management functions, which operate similarly to NEP-178 in some ways.

```rust
#[ext_contract(ext_chain_key_token_approval)]
pub trait ChainKeyTokenApproval {
    fn ckt_approve(&mut self, token_id: String, account_id: AccountId) -> u32;
    fn ckt_approve_call(
        &mut self,
        token_id: String,
        account_id: AccountId,
        msg: Option<String>,
    ) -> PromiseOrValue<Option<u32>>;
    fn ckt_revoke(&mut self, token_id: String, account_id: AccountId);
    fn ckt_revoke_call(
        &mut self,
        token_id: String,
        account_id: AccountId,
        msg: Option<String>,
    ) -> PromiseOrValue<()>;
    fn ckt_revoke_all(&mut self, token_id: String) -> near_sdk::json_types::U64;
    fn ckt_approval_id_for(&self, token_id: String, account_id: AccountId) -> Option<u32>;
}

#[ext_contract(ext_chain_key_token_approval_receiver)]
pub trait ChainKeyTokenApprovalReceiver {
    fn ckt_on_approved(
        &mut self,
        approver_id: AccountId,
        token_id: String,
        approval_id: u32,
        msg: String,
    ) -> PromiseOrValue<bool>;
    fn ckt_on_revoked(
        &mut self,
        approver_id: AccountId,
        token_id: String,
        approval_id: u32,
        msg: String,
    ) -> PromiseOrValue<()>;
}
```

#### `ckt_approve[_call]`

Issue an approval to a receiving account. This allows the account to issue signatures on behalf of this token and all of its sub-paths. Use the `_call` variant to alert the receiving contract of the approval via its `ckt_on_approved` function.

#### `ckt_revoke[_call]`

Remove an approval from an account. This prevents the account from issuing more signatures. Use the `_call` variant to alert the receiving contract of the revocation (ex post facto) via its `ckt_on_revoked` function.

#### `ckt_revoke_all`

Remove all approvals for a token. The equivalent of this function is called whenever a chain key NFT is transferred. There is no `_call` variant.
