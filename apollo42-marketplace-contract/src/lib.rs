use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, serde_json::json, AccountId, Balance,
    BorshStorageKey, CryptoHash, Gas, PanicOnDefault, Promise, Timestamp,
};
use near_sdk::{is_promise_success, promise_result_as_success};
use std::collections::HashMap;

use crate::external::*;

mod external;
mod nft_callbacks;

pub type PayoutHashMap = HashMap<AccountId, U128>;
pub type ContractAndTokenId = String;
pub type ContractAccountIdTokenId = String;
pub type TokenId = String;
pub type TokenSeriesId = String;
pub type TimestampSec = u32;

const EMPTY_BALANCE: Balance = 0;
const GAS_FOR_ROYALTIES: Gas = Gas(5_000_000_000_000 * 10u64);
const NFT_TRANSFER_PRICE: Gas = Gas(20_000_000_000_000);
const NFT_MAX_PRICE: Balance = 1_000_000_000 * 10u128.pow(24);

pub const NFT_LISTING_STORAGE_PRICE: u128 = 8590000000000000000000;

const TRADE_FIRST_CB_PRICE: Gas = Gas(30_000_000_000_000);
const TRADE_SECOND_CB_PRICE: Gas = Gas(80_000_000_000_000);

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Payout {
    pub payout: PayoutHashMap,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct TransactionFee {
    pub next_fee: Option<u16>,
    pub start_time: Option<TimestampSec>,
    pub current_fee: u16,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Bid {
    pub bidder_id: AccountId,
    pub price: U128,
}

pub type Bids = Vec<Bid>;

fn near_account() -> AccountId {
    AccountId::new_unchecked("near".to_string())
}

const DELIMETER: &str = "||";
const NEAR: &str = "near";

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MarketDataV1 {
    pub owner_id: AccountId,
    pub approval_id: u64,
    pub nft_contract_id: AccountId,
    pub token_id: TokenId,
    pub ft_token_id: AccountId,
    pub price: u128,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MarketData {
    pub owner_id: AccountId,
    pub approval_id: u64,
    pub nft_contract_id: AccountId,
    pub token_id: TokenId,
    pub ft_token_id: AccountId, // "near" for NEAR token
    pub price: u128,            // if auction, price becomes starting price
    pub bids: Option<Bids>,
    pub started_at: Option<u64>,
    pub ended_at: Option<u64>,
    pub end_price: Option<u128>, // dutch auction
    pub accept_nft_contract_id: Option<String>,
    pub accept_token_id: Option<String>,
    pub is_auction: Option<bool>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct MarketDataTransactionFee {
    pub transaction_fee: UnorderedMap<ContractAndTokenId, u128>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct OfferData {
    pub buyer_id: AccountId,
    pub nft_contract_id: AccountId,
    pub token_id: Option<TokenId>,
    pub token_series_id: Option<TokenId>,
    pub ft_token_id: AccountId, // "near" for NEAR token
    pub price: u128,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct OfferDataJson {
    buyer_id: AccountId,
    nft_contract_id: AccountId,
    token_id: Option<TokenId>,
    token_series_id: Option<TokenId>,
    ft_token_id: AccountId, // "near" for NEAR token
    price: U128,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TradeData {
    pub buyer_amount: Option<Balance>,
    pub seller_amount: Option<Balance>,
    pub ft_token_id: Option<String>,
    pub is_active: Option<bool>,
    pub nft_contract_id: AccountId,
    pub token_id: Option<TokenId>,
    pub token_series_id: Option<TokenSeriesId>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct MarketDataJson {
    owner_id: AccountId,
    approval_id: U64,
    nft_contract_id: AccountId,
    token_id: TokenId,
    ft_token_id: AccountId, // "near" for NEAR token
    price: U128,
    bids: Option<Bids>,
    started_at: Option<U64>,
    ended_at: Option<U64>,
    end_price: Option<U128>, // dutch auction
    is_auction: Option<bool>,
    transaction_fee: U128,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct TradeList {
    pub approval_id: u64,
    pub trade_data: HashMap<ContractAccountIdTokenId, TradeData>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub owner_id: AccountId,
    pub treasury_id: AccountId,
    pub old_market: UnorderedMap<ContractAndTokenId, MarketDataV1>,
    pub market: UnorderedMap<ContractAndTokenId, MarketData>,
    pub approved_ft_token_ids: UnorderedSet<AccountId>,
    pub approved_nft_contract_ids: UnorderedSet<AccountId>,
    pub storage_deposits: LookupMap<AccountId, Balance>,
    pub by_owner_id: LookupMap<AccountId, UnorderedSet<TokenId>>,
    pub offers: UnorderedMap<ContractAccountIdTokenId, OfferData>,
    pub whitelisted_nft_contracts: UnorderedSet<AccountId>,
    pub transaction_fee: TransactionFee,
    pub trades: UnorderedMap<ContractAccountIdTokenId, TradeList>,
    pub market_data_transaction_fee: MarketDataTransactionFee,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKey {
    Market,
    FTTokenIds,
    NFTContractIds,
    StorageDeposits,
    ByOwnerId,
    ByOwnerIdInner { account_id_hash: CryptoHash },
    Offers,
    WhitelistedNFTContractIds,
    MarketV2,
    Trade,
    MarketDataTransactionFee,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        owner_id: AccountId,
        treasury_id: AccountId,
        approved_ft_token_ids: Option<Vec<AccountId>>,
        approved_nft_contract_ids: Option<Vec<AccountId>>,
        whitelisted_nft_contracts: Option<Vec<AccountId>>,
        current_fee: u16,
    ) -> Self {
        let mut this = Self {
            owner_id: owner_id.into(),
            treasury_id: treasury_id.into(),
            old_market: UnorderedMap::new(StorageKey::Market),
            market: UnorderedMap::new(StorageKey::MarketV2),
            approved_ft_token_ids: UnorderedSet::new(StorageKey::FTTokenIds),
            approved_nft_contract_ids: UnorderedSet::new(StorageKey::NFTContractIds),
            storage_deposits: LookupMap::new(StorageKey::StorageDeposits),
            by_owner_id: LookupMap::new(StorageKey::ByOwnerId),
            offers: UnorderedMap::new(StorageKey::Offers),
            whitelisted_nft_contracts: UnorderedSet::new(StorageKey::WhitelistedNFTContractIds),
            transaction_fee: TransactionFee {
                next_fee: None,
                start_time: None,
                current_fee,
            },
            trades: UnorderedMap::new(StorageKey::Trade),
            market_data_transaction_fee: MarketDataTransactionFee {
                transaction_fee: UnorderedMap::new(StorageKey::MarketDataTransactionFee),
            },
        };

        this.approved_ft_token_ids.insert(&near_account());

        add_accounts(approved_ft_token_ids, &mut this.approved_ft_token_ids);
        add_accounts(
            approved_nft_contract_ids,
            &mut this.approved_nft_contract_ids,
        );
        add_accounts(whitelisted_nft_contracts, &mut this.whitelisted_nft_contracts);

        this
    }

    // #[init(ignore_state)]
    // pub fn migrate() -> Self {
    //     let prev: ContractV2 = env::state_read().expect("ERR_NOT_INITIALIZED");
    //     assert_eq!(
    //         env::predecessor_account_id(),
    //         prev.owner_id,
    //         "Only owner can migrate"
    //     );

    //     let this = Contract {
    //         owner_id: prev.owner_id,
    //         treasury_id: prev.treasury_id,
    //         old_market: prev.old_market,
    //         market: prev.market,
    //         approved_ft_token_ids: prev.approved_ft_token_ids,
    //         approved_nft_contract_ids: prev.approved_nft_contract_ids,
    //         storage_deposits: prev.storage_deposits,
    //         by_owner_id: prev.by_owner_id,
    //         offers: prev.offers,
    //         whitelisted_nft_contracts: prev.whitelisted_nft_contracts,
    //         transaction_fee: prev.transaction_fee,
    //         trades: prev.trades,
    //         market_data_transaction_fee: MarketDataTransactionFee{
    //             transaction_fee: UnorderedMap::new(StorageKey::MarketDataTransactionFee)
    //         }
    //     };

    //     this
    // }
    // Changing treasury & ownership

    #[payable]
    pub fn set_treasury(&mut self, treasury_id: AccountId) {
        assert_one_yocto();
        self.assert_owner();
        self.treasury_id = treasury_id;
    }

    #[payable]
    pub fn set_tx_fee(&mut self, next_fee: u16, start_time: Option<TimestampSec>) {
        assert_one_yocto();
        self.assert_owner();

        assert!(next_fee < 10_000, "Fee is higher than 10_000");

        if start_time.is_none() {
            self.transaction_fee.current_fee = next_fee;
            self.transaction_fee.next_fee = None;
            self.transaction_fee.start_time = None;
            return;
        } else {
            let start_time: TimestampSec = start_time.unwrap();
            assert!(
                start_time > to_sec(env::block_timestamp()),
                "start_time is less than current block_timestamp"
            );
            self.transaction_fee.next_fee = Some(next_fee);
            self.transaction_fee.start_time = Some(start_time);
        }
    }

    pub fn calculate_market_data_transaction_fee(
        &mut self,
        nft_contract_id: &AccountId,
        token_id: &TokenId,
    ) -> u128 {
        let contract_and_token_id = format!("{}{}{}", nft_contract_id, DELIMETER, token_id);
        if let Some(transaction_fee) = self
            .market_data_transaction_fee
            .transaction_fee
            .get(&contract_and_token_id)
        {
            return transaction_fee;
        }

        // fallback to default transaction fee
        self.calculate_current_transaction_fee()
    }

    pub fn calculate_current_transaction_fee(&mut self) -> u128 {
        let transaction_fee: &TransactionFee = &self.transaction_fee;
        if transaction_fee.next_fee.is_some() {
            if to_sec(env::block_timestamp()) >= transaction_fee.start_time.unwrap() {
                self.transaction_fee.current_fee = transaction_fee.next_fee.unwrap();
                self.transaction_fee.next_fee = None;
                self.transaction_fee.start_time = None;
            }
        }
        self.transaction_fee.current_fee as u128
    }

    pub fn get_transaction_fee(&self) -> &TransactionFee {
        &self.transaction_fee
    }

    pub fn get_market_data_transaction_fee(
        self,
        nft_contract_id: &AccountId,
        token_id: &TokenId,
    ) -> u128 {
        let contract_and_token_id = format!("{}{}{}", nft_contract_id, DELIMETER, token_id);
        if let Some(transaction_fee) = self
            .market_data_transaction_fee
            .transaction_fee
            .get(&contract_and_token_id)
        {
            return transaction_fee;
        }

        // fallback to default transaction fee
        self.transaction_fee.current_fee as u128
    }

    #[payable]
    pub fn change_owner(&mut self, owner_id: AccountId) {
        assert_one_yocto();
        self.assert_owner();
        self.owner_id = owner_id;
    }

    // Approved contracts
    #[payable]
    pub fn add_approved_nft_contract_ids(&mut self, nft_contract_ids: Vec<AccountId>) {
        assert_one_yocto();
        self.assert_owner();
        add_accounts(Some(nft_contract_ids), &mut self.approved_nft_contract_ids);
    }

    #[payable]
    pub fn remove_approved_nft_contract_ids(&mut self, nft_contract_ids: Vec<AccountId>) {
        assert_one_yocto();
        self.assert_owner();
        remove_accounts(Some(nft_contract_ids), &mut self.approved_nft_contract_ids);
    }

    // Whitelisted NFT contracts
    #[payable]
    pub fn add_whitelisted_nft_contract_ids(&mut self, nft_contract_ids: Vec<AccountId>) {
        assert_one_yocto();
        self.assert_owner();
        add_accounts(Some(nft_contract_ids), &mut self.whitelisted_nft_contracts);
    }

    #[payable]
    pub fn add_approved_ft_token_ids(&mut self, ft_token_ids: Vec<AccountId>) {
        assert_one_yocto();
        self.assert_owner();
        add_accounts(Some(ft_token_ids), &mut self.approved_ft_token_ids);
    }

    // Buy & Payment

    #[payable]
    pub fn buy(
        &mut self,
        nft_contract_id: AccountId,
        token_id: TokenId,
        ft_token_id: Option<AccountId>,
        price: Option<U128>,
    ) {
        let contract_and_token_id = format!("{}{}{}", &nft_contract_id, DELIMETER, token_id);
        let market_data: Option<MarketData> = if let Some(market_data) =
            self.market.get(&contract_and_token_id)
        {
            Some(market_data)
        } else {
            env::panic_str(&"Market data for the item not found. Please try to buy another one!");
        };

        let market_data: MarketData = market_data
            .expect("Market data for the item not found. Please try to buy another one!");

        let buyer_id = env::predecessor_account_id();

        assert_ne!(
            buyer_id, market_data.owner_id,
            "You can't buy your own item ;-)"
        );

        // only NEAR supported for now
        assert_eq!(
            market_data.ft_token_id.to_string(),
            NEAR,
            "Sorry, this token is not supported yet :-("
        );

        if ft_token_id.is_some() {
            assert_eq!(
                ft_token_id.unwrap().to_string(),
                market_data.ft_token_id.to_string()
            )
        }
        if price.is_some() {
            assert_eq!(price.unwrap().0, market_data.price);
        }

        let mut price = market_data.price;

        if market_data.is_auction.is_some() && market_data.end_price.is_some() {
            let current_time = env::block_timestamp();
            let end_price = market_data.end_price.unwrap();
            let ended_at = market_data.ended_at.unwrap();
            let started_at = market_data.started_at.unwrap();

            assert!(
                current_time >= started_at,
                "The auction has not started yet."
            );

            if current_time > ended_at {
                price = end_price;
            } else {
                let time_since_start = current_time - started_at;
                let duration = ended_at - started_at;
                price = price - ((price - end_price) / duration as u128) * time_since_start as u128;
            }
        } else if let Some(auction) = market_data.is_auction {
            assert_eq!(auction, false, "The auction has not ended yet.");
        }

        assert!(
            env::attached_deposit() >= price,
            "The attached deposit is less than the item price {}. Please consider increasing it :)",
            price
        );

        self.internal_process_purchase(nft_contract_id.into(), token_id, buyer_id, price);
    }

    fn internal_process_purchase(
        &mut self,
        nft_contract_id: AccountId,
        token_id: TokenId,
        buyer_id: AccountId,
        price: u128,
    ) -> Promise {
        let market_data = self
            .internal_delete_market_data(&nft_contract_id, &token_id)
            .expect("The sale does not exist");

        ext_contract::nft_transfer_payout(
            buyer_id.clone(),
            token_id,
            Some(market_data.approval_id),
            Some(price.into()),
            Some(10u32), // max length payout
            nft_contract_id,
            1,
            NFT_TRANSFER_PRICE,
        )
        .then(ext_self::resolve_purchase(
            buyer_id,
            market_data,
            price.into(),
            env::current_account_id(),
            EMPTY_BALANCE,
            GAS_FOR_ROYALTIES,
        ))
    }

    #[private]
    pub fn resolve_purchase(
        &mut self,
        buyer_id: AccountId,
        market_data: MarketData,
        price: U128,
    ) -> U128 {
        let payout_option = promise_result_as_success().and_then(|value| {
            let parsed_payout = near_sdk::serde_json::from_slice::<PayoutHashMap>(&value);
            if parsed_payout.is_err() {
                near_sdk::serde_json::from_slice::<Payout>(&value)
                    .ok()
                    .and_then(|payout| {
                        let mut remainder = price.0;
                        for &value in payout.payout.values() {
                            remainder = remainder.checked_sub(value.0)?;
                        }
                        if remainder <= 100 {
                            Some(payout.payout)
                        } else {
                            None
                        }
                    })
            } else {
                parsed_payout.ok().and_then(|payout| {
                    let mut remainder = price.0;
                    for &value in payout.values() {
                        remainder = remainder.checked_sub(value.0)?;
                    }
                    if remainder <= 100 {
                        Some(payout)
                    } else {
                        None
                    }
                })
            }
        });
        let payout = if let Some(payout_option) = payout_option {
            payout_option
        } else {
            // leave function and return all FTs in ft_resolve_transfer
            if !is_promise_success() {
                if market_data.ft_token_id == near_account() {
                    Promise::new(buyer_id.clone()).transfer(u128::from(market_data.price));
                }
                env::log_str(
                    &json!({
                        "type": "resolve_purchase_fail",
                        "params": {
                            "owner_id": market_data.owner_id,
                            "nft_contract_id": market_data.nft_contract_id,
                            "token_id": market_data.token_id,
                            "ft_token_id": market_data.ft_token_id,
                            "price": price,
                            "buyer_id": buyer_id,
                        }
                    })
                    .to_string(),
                );
            } else if market_data.ft_token_id == near_account() {
                let treasury_fee = price.0
                    * self.calculate_market_data_transaction_fee(
                        &market_data.nft_contract_id,
                        &market_data.token_id,
                    )
                    / 10_000u128;
                let contract_and_token_id = format!(
                    "{}{}{}",
                    &market_data.nft_contract_id, DELIMETER, &market_data.token_id
                );
                self.market_data_transaction_fee
                    .transaction_fee
                    .remove(&contract_and_token_id);
                Promise::new(market_data.owner_id.clone()).transfer(price.0 - treasury_fee);
                if treasury_fee > 0 {
                    Promise::new(self.treasury_id.clone()).transfer(treasury_fee);
                }

                env::log_str(
                    &json!({
                        "type": "resolve_purchase",
                        "params": {
                            "owner_id": &market_data.owner_id,
                            "nft_contract_id": &market_data.nft_contract_id,
                            "token_id": &market_data.token_id,
                            "ft_token_id": market_data.ft_token_id,
                            "price": price,
                            "buyer_id": buyer_id,
                        }
                    })
                    .to_string(),
                );
            }
            return price;
        };

        // Payout (transfer to royalties and seller)
        if market_data.ft_token_id == near_account() {
            // 5% fee for treasury
            let treasury_fee = price.0
                * self.calculate_market_data_transaction_fee(
                    &market_data.nft_contract_id,
                    &market_data.token_id,
                )
                / 10_000u128;
            let contract_and_token_id = format!(
                "{}{}{}",
                &market_data.nft_contract_id, DELIMETER, &market_data.token_id
            );
            self.market_data_transaction_fee
                .transaction_fee
                .remove(&contract_and_token_id);

            for (receiver_id, amount) in payout {
                if receiver_id == market_data.owner_id {
                    Promise::new(receiver_id).transfer(amount.0 - treasury_fee);
                    if treasury_fee != 0 {
                        Promise::new(self.treasury_id.clone()).transfer(treasury_fee);
                    }
                } else {
                    Promise::new(receiver_id).transfer(amount.0);
                }
            }
            env::log_str(
                &json!({
                    "type": "resolve_purchase",
                    "params": {
                        "owner_id": &market_data.owner_id,
                        "nft_contract_id": &market_data.nft_contract_id,
                        "token_id": &market_data.token_id,
                        "ft_token_id": market_data.ft_token_id,
                        "price": price,
                        "buyer_id": buyer_id,
                    }
                })
                .to_string(),
            );

            let seller_contract_account_id_token_id = make_triple(
                &market_data.nft_contract_id,
                &market_data.owner_id,
                &market_data.token_id,
            );
            self.trades.remove(&seller_contract_account_id_token_id);

            return price;
        } else {
            U128(0)
        }
    }

    // Offer

    fn internal_add_offer(
        &mut self,
        nft_contract_id: AccountId,
        token_id: Option<TokenId>,
        token_series_id: Option<TokenId>,
        ft_token_id: AccountId,
        price: U128,
        buyer_id: AccountId,
    ) {
        let token = if token_id.is_some() {
            token_id.as_ref().unwrap().to_string()
        } else {
            token_series_id.as_ref().unwrap().to_string()
        };

        let contract_account_id_token_id = make_triple(&nft_contract_id, &buyer_id, &token);
        self.offers.insert(
            &contract_account_id_token_id,
            &OfferData {
                buyer_id: buyer_id.clone().into(),
                nft_contract_id: nft_contract_id.into(),
                token_id: token_id,
                token_series_id: token_series_id,
                ft_token_id: ft_token_id.into(),
                price: price.into(),
            },
        );

        let mut token_ids = self.by_owner_id.get(&buyer_id).unwrap_or_else(|| {
            UnorderedSet::new(
                StorageKey::ByOwnerIdInner {
                    account_id_hash: hash_account_id(&buyer_id),
                }
                .try_to_vec()
                .unwrap(),
            )
        });
        token_ids.insert(&contract_account_id_token_id);
        self.by_owner_id.insert(&buyer_id, &token_ids);
    }

    #[payable]
    pub fn add_offer(
        &mut self,
        nft_contract_id: AccountId,
        token_id: Option<TokenId>,
        token_series_id: Option<String>,
        ft_token_id: AccountId,
        price: U128,
    ) {
        let token = if token_id.is_some() {
            token_id.as_ref().unwrap().to_string()
        } else {
            assert!(
                self.whitelisted_nft_contracts.contains(&nft_contract_id),
                "Offers for series are available only for NFTs minted on Apollo42."
            );
            token_series_id.as_ref().unwrap().to_string()
        };

        assert_eq!(
            env::attached_deposit(),
            price.0,
            "The attached deposit is not equal to the item price."
        );

        assert_eq!(
            ft_token_id.to_string(),
            "near",
            "Only NEAR tokens are supported currently."
        );

        let buyer_id = env::predecessor_account_id();
        let offer_data = self.internal_delete_offer(
            nft_contract_id.clone().into(),
            buyer_id.clone(),
            token.clone(),
        );

        if offer_data.is_some() {
            Promise::new(buyer_id.clone()).transfer(offer_data.unwrap().price);
        }

        let storage_amount = self.storage_minimum_balance().0;
        let owner_paid_storage = self.storage_deposits.get(&buyer_id).unwrap_or(0);
        let signer_storage_required =
            (self.get_supply_by_owner_id(buyer_id.clone()).0 + 1) as u128 * storage_amount;

        assert!(
            owner_paid_storage >= signer_storage_required,
            "Insufficient storage paid: {}, for {} offer at {} rate of per offer",
            owner_paid_storage,
            signer_storage_required / storage_amount,
            storage_amount,
        );

        self.internal_add_offer(
            nft_contract_id.clone().into(),
            token_id.clone(),
            token_series_id.clone(),
            ft_token_id.clone(),
            price,
            buyer_id.clone(),
        );

        env::log_str(
            &json!({
                "type": "add_offer",
                "params": {
                    "buyer_id": buyer_id,
                    "nft_contract_id": nft_contract_id,
                    "token_id": token_id,
                    "token_series_id": token_series_id,
                    "ft_token_id": ft_token_id,
                    "price": price,
                }
            })
            .to_string(),
        );
    }

    fn internal_delete_offer(
        &mut self,
        nft_contract_id: AccountId,
        buyer_id: AccountId,
        token_id: TokenId,
    ) -> Option<OfferData> {
        let contract_account_id_token_id = make_triple(&nft_contract_id, &buyer_id, &token_id);
        let offer_data = self.offers.remove(&contract_account_id_token_id);

        match offer_data {
            Some(offer) => {
                let by_owner_id = self.by_owner_id.get(&offer.buyer_id);
                if let Some(mut by_owner_id) = by_owner_id {
                    by_owner_id.remove(&contract_account_id_token_id);
                    if by_owner_id.is_empty() {
                        self.by_owner_id.remove(&offer.buyer_id);
                    } else {
                        self.by_owner_id.insert(&offer.buyer_id, &by_owner_id);
                    }
                }
                return Some(offer);
            }
            None => return None,
        };
    }

    #[payable]
    pub fn delete_offer(
        &mut self,
        nft_contract_id: AccountId,
        token_id: Option<TokenId>,
        token_series_id: Option<String>,
    ) {
        assert_one_yocto();
        let token = if token_id.is_some() {
            token_id.as_ref().unwrap().to_string()
        } else {
            token_series_id.as_ref().unwrap().to_string()
        };

        let buyer_id = env::predecessor_account_id();
        let contract_account_id_token_id = make_triple(&nft_contract_id, &buyer_id, &token);

        let offer_data = self
            .offers
            .get(&contract_account_id_token_id)
            .expect("Offer does not exist");

        if token_id.is_some() {
            assert_eq!(offer_data.token_id.unwrap(), token)
        } else {
            assert_eq!(offer_data.token_series_id.unwrap(), token)
        }

        assert_eq!(
            offer_data.buyer_id, buyer_id,
            "You can't delete offers you didn't make."
        );

        self.internal_delete_offer(
            nft_contract_id.clone().into(),
            buyer_id.clone(),
            token.clone(),
        )
        .expect("Offer not found");

        Promise::new(offer_data.buyer_id).transfer(offer_data.price);

        env::log_str(
            &json!({
                "type": "delete_offer",
                "params": {
                    "nft_contract_id": nft_contract_id,
                    "buyer_id": buyer_id,
                    "token_id": token_id,
                    "token_series_id": token_series_id,
                }
            })
            .to_string(),
        );
    }

    pub fn get_offer(
        &self,
        nft_contract_id: AccountId,
        buyer_id: AccountId,
        token_id: Option<TokenId>,
        token_series_id: Option<String>,
    ) -> OfferDataJson {
        let token = if token_id.is_some() {
            token_id.as_ref().unwrap()
        } else {
            token_series_id.as_ref().unwrap()
        };

        let contract_account_id_token_id = make_triple(&nft_contract_id, &buyer_id, &token);

        let offer_data = self
            .offers
            .get(&contract_account_id_token_id)
            .expect("Offer does not exist");

        if token_id.is_some() {
            assert_eq!(offer_data.token_id.as_ref().unwrap(), token);
        } else {
            assert_eq!(offer_data.token_series_id.as_ref().unwrap(), token);
        }

        OfferDataJson {
            buyer_id: offer_data.buyer_id,
            nft_contract_id: offer_data.nft_contract_id,
            token_id: offer_data.token_id,
            token_series_id: offer_data.token_series_id,
            ft_token_id: offer_data.ft_token_id,
            price: U128(offer_data.price),
        }
    }

    fn internal_accept_offer(
        &mut self,
        nft_contract_id: AccountId,
        buyer_id: AccountId,
        token_id: TokenId,
        seller_id: AccountId,
        approval_id: u64,
        price: u128,
    ) -> Promise {
        let contract_account_id_token_id = make_triple(&nft_contract_id, &buyer_id, &token_id);

        self.internal_delete_market_data(&nft_contract_id, &token_id);

        let offer_data = self
            .offers
            .get(&contract_account_id_token_id)
            .expect("Offer does not exist");

        assert_eq!(offer_data.token_id.as_ref().unwrap(), &token_id);
        assert_eq!(offer_data.price, price);

        let offer_data = self
            .internal_delete_offer(
                nft_contract_id.clone().into(),
                buyer_id.clone(),
                token_id.clone(),
            )
            .expect("Offer does not exist");

        ext_contract::nft_transfer_payout(
            offer_data.buyer_id.clone(),
            token_id.clone(),
            Some(approval_id),
            Some(U128::from(offer_data.price)),
            Some(10u32), // max length payout
            nft_contract_id,
            1,
            NFT_TRANSFER_PRICE,
        )
        .then(ext_self::resolve_offer(
            seller_id,
            offer_data,
            token_id,
            env::current_account_id(),
            EMPTY_BALANCE,
            GAS_FOR_ROYALTIES,
        ))
    }

    fn internal_accept_offer_series(
        &mut self,
        nft_contract_id: AccountId,
        buyer_id: AccountId,
        token_id: TokenId,
        seller_id: AccountId,
        approval_id: u64,
        price: u128,
    ) -> Promise {
        // Token delimiter : is specific for NFTs minted using Apollo42 Token Minter

        let mut token_id_iter = token_id.split(":");
        let token_series_id: String = token_id_iter.next().unwrap().parse().unwrap();

        let contract_account_id_token_id =
            make_triple(&nft_contract_id, &buyer_id, &token_series_id);

        self.internal_delete_market_data(&nft_contract_id, &token_id);

        let offer_data = self
            .offers
            .get(&contract_account_id_token_id)
            .expect("Offer does not exist");

        assert_eq!(
            offer_data.token_series_id.as_ref().unwrap(),
            &token_series_id
        );
        assert_eq!(offer_data.price, price);

        self.internal_delete_offer(
            nft_contract_id.clone().into(),
            buyer_id.clone(),
            token_series_id.clone(),
        )
        .expect("Offer does not exist");

        ext_contract::nft_transfer_payout(
            offer_data.buyer_id.clone(),
            token_id.clone(),
            Some(approval_id),
            Some(U128::from(offer_data.price)),
            Some(10u32), // max length payout
            nft_contract_id,
            1,
            NFT_TRANSFER_PRICE,
        )
        .then(ext_self::resolve_offer(
            seller_id,
            offer_data,
            token_id,
            env::current_account_id(),
            EMPTY_BALANCE,
            GAS_FOR_ROYALTIES,
        ))
    }

    #[private]
    pub fn resolve_offer(
        &mut self,
        seller_id: AccountId,
        offer_data: OfferData,
        token_id: TokenId,
    ) -> U128 {
        let payout_option = promise_result_as_success().and_then(|value| {
            // None means a bad payout from bad NFT contract
            let parsed_payout = near_sdk::serde_json::from_slice::<PayoutHashMap>(&value);
            if parsed_payout.is_err() {
                near_sdk::serde_json::from_slice::<Payout>(&value)
                    .ok()
                    .and_then(|payout| {
                        let mut remainder = offer_data.price;
                        for &value in payout.payout.values() {
                            remainder = remainder.checked_sub(value.0)?;
                        }
                        if remainder <= 100 {
                            Some(payout.payout)
                        } else {
                            None
                        }
                    })
            } else {
                parsed_payout.ok().and_then(|payout| {
                    let mut remainder = offer_data.price;
                    for &value in payout.values() {
                        remainder = remainder.checked_sub(value.0)?;
                    }
                    if remainder <= 100 {
                        Some(payout)
                    } else {
                        None
                    }
                })
            }
        });

        let payout = if let Some(payout_option) = payout_option {
            payout_option
        } else {
            if !is_promise_success() {
                if offer_data.ft_token_id == near_account() {
                    Promise::new(offer_data.buyer_id.clone())
                        .transfer(u128::from(offer_data.price));
                    env::log_str(
                        &json!({
                            "type": "resolve_purchase_fail",
                            "params": {
                                "owner_id": seller_id,
                                "nft_contract_id": offer_data.nft_contract_id,
                                "token_id": token_id,
                                "token_series_id": offer_data.token_series_id,
                                "ft_token_id": offer_data.ft_token_id,
                                "price": offer_data.price.to_string(),
                                "buyer_id": offer_data.buyer_id,
                                "is_offer": true,
                            }
                        })
                        .to_string(),
                    );
                }
            } else if offer_data.ft_token_id == near_account() {
                let treasury_fee = offer_data.price as u128
                    * self.calculate_current_transaction_fee()
                    / 10_000u128;
                Promise::new(seller_id.clone()).transfer(offer_data.price - treasury_fee);
                if treasury_fee > 0 {
                    Promise::new(self.treasury_id.clone()).transfer(treasury_fee);
                }

                env::log_str(
                    &json!({
                        "type": "resolve_purchase",
                        "params": {
                            "owner_id": seller_id,
                            "nft_contract_id": &offer_data.nft_contract_id,
                            "token_id": &token_id,
                            "token_series_id": offer_data.token_series_id,
                            "ft_token_id": offer_data.ft_token_id,
                            "price": offer_data.price.to_string(),
                            "buyer_id": offer_data.buyer_id,
                            "is_offer": true,
                        }
                    })
                    .to_string(),
                );
            }
            return offer_data.price.into();
        };

        // Payout (transfer to royalties and seller)
        if offer_data.ft_token_id == near_account() {
            // 5% fee for treasury
            let treasury_fee =
                offer_data.price as u128 * self.calculate_current_transaction_fee() / 10_000u128;

            for (receiver_id, amount) in payout {
                if receiver_id == seller_id {
                    Promise::new(receiver_id).transfer(amount.0 - treasury_fee);
                    if treasury_fee != 0 {
                        Promise::new(self.treasury_id.clone()).transfer(treasury_fee);
                    }
                } else {
                    Promise::new(receiver_id).transfer(amount.0);
                }
            }

            env::log_str(
                &json!({
                    "type": "resolve_purchase",
                    "params": {
                        "owner_id": seller_id,
                        "nft_contract_id": &offer_data.nft_contract_id,
                        "token_id": &token_id,
                        "token_series_id": offer_data.token_series_id,
                        "ft_token_id": offer_data.ft_token_id,
                        "price": offer_data.price.to_string(),
                        "buyer_id": offer_data.buyer_id,
                        "is_offer": true,
                    }
                })
                .to_string(),
            );

            let seller_contract_account_id_token_id =
                make_triple(&offer_data.nft_contract_id, &seller_id, &token_id);
            self.trades.remove(&seller_contract_account_id_token_id);

            return offer_data.price.into();
        } else {
            U128(0)
        }
    }

    // Trade
    fn add_trade(
        &mut self,
        nft_contract_id: AccountId,
        token_id: Option<TokenId>,
        token_series_id: Option<TokenSeriesId>,
        buyer_nft_contract_id: AccountId,
        buyer_id: AccountId,
        buyer_token_id: Option<TokenId>,
        buyer_approval_id: u64,
    ) {
        self.internal_add_trade(
            nft_contract_id.clone().into(),
            token_id.clone(),
            token_series_id.clone(),
            buyer_nft_contract_id.clone().into(),
            buyer_token_id.clone(),
            buyer_id.clone(),
            buyer_approval_id.clone(),
        );

        env::log_str(
            &json!({
                "type": "add_trade",
                "params": {
                    "buyer_id": buyer_id,
                    "nft_contract_id": nft_contract_id,
                    "token_id": token_id,
                    "token_series_id": token_series_id,
                    "buyer_nft_contract_id": buyer_nft_contract_id,
                    "buyer_token_id": buyer_token_id,
                    "buyer_approval_id": buyer_approval_id
                }
            })
            .to_string(),
        );
    }

    fn internal_add_trade(
        &mut self,
        nft_contract_id: AccountId,
        token_id: Option<TokenId>,
        token_series_id: Option<TokenSeriesId>,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: Option<TokenId>,
        buyer_id: AccountId,
        buyer_approval_id: u64,
    ) {
        let token = if token_id.is_some() {
            token_id.as_ref().unwrap().to_string()
        } else {
            assert!(
                self.whitelisted_nft_contracts.contains(&nft_contract_id),
                "Offers for series are available only for NFTs minted on Apollo42."
            );
            token_series_id.as_ref().unwrap().to_string()
        };

        let contract_account_id_token_id = make_triple(&nft_contract_id, &buyer_id, &token);
        let buyer_contract_account_id_token_id = make_triple(
            &buyer_nft_contract_id,
            &buyer_id,
            &buyer_token_id
                .as_ref()
                .expect("Buyer TokenId is not specified"),
        );

        let trade_data = TradeData {
            buyer_amount: None,
            seller_amount: None,
            is_active: None,
            ft_token_id: None,
            nft_contract_id: nft_contract_id.into(),
            token_id: token_id,
            token_series_id: token_series_id,
        };
        let mut buyer_trade_list = self
            .trades
            .get(&buyer_contract_account_id_token_id)
            .unwrap_or_else(|| {
                TradeList {
                    approval_id: 0, //init
                    trade_data: HashMap::new(),
                }
            });
        buyer_trade_list.approval_id = buyer_approval_id;
        buyer_trade_list
            .trade_data
            .insert(contract_account_id_token_id.clone(), trade_data);

        self.trades
            .insert(&buyer_contract_account_id_token_id, &buyer_trade_list);

        let mut token_ids = self.by_owner_id.get(&buyer_id).unwrap_or_else(|| {
            UnorderedSet::new(
                StorageKey::ByOwnerIdInner {
                    account_id_hash: hash_account_id(&buyer_id),
                }
                .try_to_vec()
                .unwrap(),
            )
        });

        token_ids.insert(&make_key_owner_by_id_trade(contract_account_id_token_id));
        self.by_owner_id.insert(&buyer_id, &token_ids);
    }

    #[payable]
    pub fn delete_trade(
        &mut self,
        nft_contract_id: AccountId,
        token_id: Option<TokenId>,
        token_series_id: Option<TokenSeriesId>,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: TokenId,
    ) {
        assert_one_yocto();
        let token = if token_id.is_some() {
            token_id.as_ref().unwrap().to_string()
        } else {
            token_series_id.as_ref().unwrap().to_string()
        };

        let buyer_id = env::predecessor_account_id();
        let buyer_contract_account_id_token_id =
            make_triple(&buyer_nft_contract_id, &buyer_id, &buyer_token_id);
        let contract_account_id_token_id = make_triple(&nft_contract_id, &buyer_id, &token);

        let trade_list = self
            .trades
            .get(&buyer_contract_account_id_token_id)
            .expect("Trade list does not exist");

        let trade_data = trade_list
            .trade_data
            .get(&contract_account_id_token_id)
            .expect("Trade data not found in trade list");

        if token_id.is_some() {
            assert_eq!(trade_data.clone().token_id.unwrap(), token)
        } else {
            assert_eq!(trade_data.clone().token_series_id.unwrap(), token)
        }

        self.internal_delete_trade(
            nft_contract_id.clone().into(),
            buyer_id.clone(),
            token.clone(),
            buyer_nft_contract_id.clone(),
            buyer_token_id.clone(),
        )
        .expect("Trade not found");

        env::log_str(
            &json!({
                "type": "delete_trade",
                "params": {
                    "nft_contract_id": nft_contract_id,
                    "buyer_id": buyer_id,
                    "token_id": token_id,
                    "token_series_id": token_series_id,
                    "buyer_nft_contract_id": buyer_nft_contract_id,
                    "buyer_token_id": buyer_token_id
                }
            })
            .to_string(),
        );
    }

    fn internal_delete_trade(
        &mut self,
        nft_contract_id: AccountId,
        buyer_id: AccountId,
        token_id: TokenId,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: TokenId,
    ) -> Option<TradeData> {
        let buyer_contract_account_id_token_id =
            make_triple(&buyer_nft_contract_id, &buyer_id, &buyer_token_id);
        let contract_account_id_token_id = make_triple(&nft_contract_id, &buyer_id, &token_id);

        let mut trade_list = self
            .trades
            .get(&buyer_contract_account_id_token_id)
            .expect("Trade list does not exist");

        let trade_data = trade_list.trade_data.remove(&contract_account_id_token_id);

        self.trades
            .insert(&buyer_contract_account_id_token_id, &trade_list);

        match trade_data {
            Some(trade) => {
                let mut by_owner_id = self
                    .by_owner_id
                    .get(&buyer_id)
                    .expect("No market data by account_id");
                by_owner_id.remove(&make_key_owner_by_id_trade(contract_account_id_token_id));
                if by_owner_id.is_empty() {
                    self.by_owner_id.remove(&buyer_id);
                } else {
                    self.by_owner_id.insert(&buyer_id, &by_owner_id);
                }
                return Some(trade);
            }
            None => {
                self.trades
                    .remove(&buyer_contract_account_id_token_id)
                    .expect("Error delete trade list");
                return None;
            }
        };
    }

    pub fn get_trade(
        &self,
        seller_nft_contract_id: AccountId,
        seller_token_id: Option<TokenId>,
        seller_token_series_id: Option<String>,
        buyer_id: AccountId,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: TokenId,
    ) -> TradeData {
        let token = if seller_token_id.is_some() {
            seller_token_id.as_ref().unwrap()
        } else {
            seller_token_series_id.as_ref().unwrap()
        };

        let contract_account_id_token_id = make_triple(&seller_nft_contract_id, &buyer_id, &token);
        let buyer_contract_account_id_token_id =
            make_triple(&buyer_nft_contract_id, &buyer_id, &buyer_token_id);

        let trade_list = self
            .trades
            .get(&buyer_contract_account_id_token_id)
            .expect("Trade list does not exist");

        let trade_data = trade_list
            .trade_data
            .get(&contract_account_id_token_id)
            .expect("Trade data does not exist");

        if seller_token_id.is_some() {
            assert_eq!(trade_data.token_id.as_ref().unwrap(), token);
        } else {
            assert_eq!(trade_data.token_series_id.as_ref().unwrap(), token);
        }

        return trade_data.clone();
    }

    fn internal_accept_trade(
        &mut self,
        nft_contract_id: AccountId,
        buyer_id: AccountId,
        token_id: TokenId,
        seller_id: AccountId,
        approval_id: u64,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: TokenId,
    ) -> Promise {
        let buyer_contract_account_id_token_id =
            make_triple(&buyer_nft_contract_id, &buyer_id, &buyer_token_id);
        let contract_account_id_token_id = make_triple(&nft_contract_id, &buyer_id, &token_id);

        let trade_list = self
            .trades
            .get(&buyer_contract_account_id_token_id)
            .expect("Trade list does not exist");

        let trade_data = trade_list
            .trade_data
            .get(&contract_account_id_token_id)
            .expect("Trade data does not exist");

        assert_eq!(trade_data.token_id.as_ref().unwrap(), &token_id);

        self.internal_delete_market_data(&nft_contract_id, &token_id);
        self.internal_delete_market_data(&buyer_nft_contract_id, &buyer_token_id);

        let seller_contract_account_id_token_id =
            make_triple(&nft_contract_id, &seller_id, &token_id);

        if let Some(mut trades) = self.trades.get(&seller_contract_account_id_token_id) {
            trades.trade_data.clear();
        }
        if let Some(mut trades) = self.trades.get(&buyer_contract_account_id_token_id) {
            trades.trade_data.clear();
        }
        self.trades.remove(&seller_contract_account_id_token_id);
        self.trades.remove(&buyer_contract_account_id_token_id);

        self.trade_swap_nft(
            buyer_id,
            buyer_nft_contract_id,
            buyer_token_id,
            trade_list.approval_id,
            seller_id,
            nft_contract_id,
            token_id,
            approval_id,
        )
    }

    fn internal_accept_trade_series(
        &mut self,
        nft_contract_id: AccountId,
        buyer_id: AccountId,
        token_id: TokenId,
        seller_id: AccountId,
        approval_id: u64,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: TokenId,
    ) -> Promise {
        // Token delimiter : is specific for NFTs minted using Apollo42 Token Minter
        let mut token_id_iter = token_id.split(":");
        let token_series_id: String = token_id_iter.next().unwrap().parse().unwrap();

        let buyer_contract_account_id_token_id =
            make_triple(&buyer_nft_contract_id, &buyer_id, &buyer_token_id);
        let contract_account_id_token_id =
            make_triple(&nft_contract_id, &buyer_id, &token_series_id);

        let trade_list = self
            .trades
            .get(&buyer_contract_account_id_token_id)
            .expect("Trade list does not exist");

        let trade_data = trade_list
            .trade_data
            .get(&contract_account_id_token_id)
            .expect("Trade data does not exist");

        assert_eq!(
            trade_data.token_series_id.as_ref().unwrap(),
            &token_series_id
        );

        self.internal_delete_market_data(&nft_contract_id, &token_id);
        self.internal_delete_market_data(&buyer_nft_contract_id, &buyer_token_id);

        let seller_contract_account_id_token_id =
            make_triple(&nft_contract_id, &seller_id, &token_id);
        self.trades.remove(&seller_contract_account_id_token_id);
        self.trades.remove(&buyer_contract_account_id_token_id);

        self.trade_swap_nft(
            buyer_id,
            buyer_nft_contract_id,
            buyer_token_id,
            trade_list.approval_id,
            seller_id,
            nft_contract_id,
            token_id,
            approval_id,
        )
    }

    fn trade_swap_nft(
        &mut self,
        buyer_id: AccountId,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: TokenId,
        buyer_approval_id: u64,
        seller_id: AccountId,
        seller_nft_contract_id: AccountId,
        seller_token_id: TokenId,
        seller_approval_id: u64,
    ) -> Promise {
        // 1. transfer buyer & seller NFT to marketplace
        // 2. verify that those NFTs is valid and has approval_id
        // 3. if those NFTs is valid then swap token to buyer & seller
        // 4. if failed then rollback the NFT to buyer or seller

        ext_contract::nft_transfer(
            env::current_account_id(),
            buyer_token_id.clone(),
            Some(buyer_approval_id),
            buyer_nft_contract_id.clone(),
            1,
            NFT_TRANSFER_PRICE,
        )
        .then(ext_self::callback_first_trade(
            seller_nft_contract_id.clone(),
            seller_token_id.clone(),
            seller_approval_id,
            env::current_account_id(),
            EMPTY_BALANCE,
            TRADE_FIRST_CB_PRICE,
        ))
        .then(ext_self::callback_second_trade(
            buyer_id,
            buyer_nft_contract_id.clone(),
            buyer_token_id.clone(),
            seller_id,
            seller_nft_contract_id.clone(),
            seller_token_id.clone(),
            env::current_account_id(),
            EMPTY_BALANCE,
            TRADE_SECOND_CB_PRICE,
        ))
    }

    #[private]
    pub fn callback_first_trade(
        &mut self,
        seller_nft_contract_id: AccountId,
        seller_token_id: TokenId,
        seller_approval_id: u64,
    ) -> Promise {
        if !is_promise_success() {
            env::panic_str(&"Buyer's nft failed to trade");
        } else {
            return ext_contract::nft_transfer(
                env::current_account_id(),
                seller_token_id.clone(),
                Some(seller_approval_id),
                seller_nft_contract_id.clone(),
                1,
                NFT_TRANSFER_PRICE,
            );
        }
    }

    #[private]
    pub fn callback_second_trade(
        &mut self,
        buyer_id: AccountId,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: TokenId,
        seller_id: AccountId,
        seller_nft_contract_id: AccountId,
        seller_token_id: TokenId,
    ) {
        if !is_promise_success() {
            ext_contract::nft_transfer(
                buyer_id,
                buyer_token_id,
                None,
                buyer_nft_contract_id,
                1,
                NFT_TRANSFER_PRICE,
            );
            env::panic_str(&"Seller's nft failed to trade, rollback buyer's nft");
        } else {
            self.internal_swap_nft(
                buyer_id,
                buyer_nft_contract_id,
                buyer_token_id,
                seller_id,
                seller_nft_contract_id,
                seller_token_id,
            );
        }
    }

    fn internal_swap_nft(
        &mut self,
        buyer_id: AccountId,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: TokenId,
        seller_id: AccountId,
        seller_nft_contract_id: AccountId,
        seller_token_id: TokenId,
    ) {
        ext_contract::nft_transfer(
            seller_id.clone(),
            buyer_token_id.clone(),
            None,
            buyer_nft_contract_id.clone(),
            1,
            NFT_TRANSFER_PRICE,
        )
        .then(ext_contract::nft_transfer(
            buyer_id.clone(),
            seller_token_id.clone(),
            None,
            seller_nft_contract_id.clone(),
            1,
            NFT_TRANSFER_PRICE,
        ));

        env::log_str(
            &json!({
                "type": "accept_trade",
                "params": {
                    "sender_id": seller_id,
                    "buyer_id": buyer_id,
                    "nft_contract_id": seller_nft_contract_id,
                    "token_id": seller_token_id,
                    "buyer_nft_contract_id": buyer_nft_contract_id,
                    "buyer_token_id": buyer_token_id,
                }
            })
            .to_string(),
        );
    }

    // Auction bids
    #[payable]
    pub fn add_bid(
        &mut self,
        nft_contract_id: AccountId,
        ft_token_id: AccountId,
        token_id: TokenId,
        amount: U128,
    ) {
        let contract_and_token_id = format!("{}{}{}", &nft_contract_id, DELIMETER, token_id);
        let mut market_data = self
            .market
            .get(&contract_and_token_id)
            .expect("Token Id does not exist");

        let bidder_id = env::predecessor_account_id();

        let current_time = env::block_timestamp();
        if market_data.started_at.is_some() {
            assert!(
                current_time >= market_data.started_at.unwrap(),
                "Sale has not started yet"
            );
        }

        if market_data.ended_at.is_some() {
            assert!(
                current_time <= market_data.ended_at.unwrap(),
                "Sale has ended"
            );
        }

        assert!(
            env::attached_deposit() >= amount.into(),
            "Attached deposit is less than the item price"
        );

        assert_eq!(ft_token_id.to_string(), "near", "Only NEAR tokens are supported currently.");

        assert!(
            market_data.end_price.is_none(),
            "Dutch auction does not accept add_bid"
        );

        let new_bid = Bid {
            bidder_id: bidder_id.clone(),
            price: amount.into(),
        };

        let mut bids = market_data.bids.unwrap_or(Vec::new());

        if !bids.is_empty() {
            let current_bid = &bids[bids.len() - 1];

            assert!(
                amount.0 > current_bid.price.0,
                "Can't pay less than or equal to current bid price: {:?}",
                current_bid.price
            );

            assert!(
                amount.0 > market_data.price,
                "Can't pay less than or equal to starting price: {:?}",
                U128(market_data.price)
            );

            // refund
            Promise::new(current_bid.bidder_id.clone()).transfer(current_bid.price.0);

            // always keep 1 bid for now
            bids.remove(bids.len() - 1);
        } else {
            assert!(
                amount.0 > market_data.price,
                "Can't pay less than or equal to starting price: {}",
                market_data.price
            );
        }

        bids.push(new_bid);
        market_data.bids = Some(bids);
        self.market.insert(&contract_and_token_id, &market_data);

        env::log_str(
            &json!({
                "type": "add_bid",
                "params": {
                    "bidder_id": bidder_id,
                    "nft_contract_id": nft_contract_id,
                    "token_id": token_id,
                    "ft_token_id": ft_token_id,
                    "amount": amount,
                }
            })
            .to_string(),
        );
    }

    #[payable]
    pub fn accept_bid(&mut self, nft_contract_id: AccountId, token_id: TokenId) {
        assert_one_yocto();
        let contract_and_token_id = format!("{}{}{}", &nft_contract_id, DELIMETER, token_id);
        let mut market_data = self
            .market
            .get(&contract_and_token_id)
            .expect("Token Id does not exist");

        assert_eq!(
            market_data.owner_id,
            env::predecessor_account_id(),
            "Only seller can call accept_bid"
        );

        assert!(
            market_data.end_price.is_none(),
            "Dutch auction does not accept accept_bid"
        );

        let mut bids = market_data.bids.unwrap();
        let selected_bid = bids.remove(bids.len() - 1);
        market_data.bids = Some(bids);
        self.market.insert(&contract_and_token_id, &market_data);

        self.internal_process_purchase(
            market_data.nft_contract_id,
            token_id,
            selected_bid.bidder_id.clone(),
            selected_bid.price.clone().0,
        );
    }

    // Market Data functions

    #[payable]
    pub fn update_market_data(
        &mut self,
        nft_contract_id: AccountId,
        token_id: TokenId,
        ft_token_id: AccountId,
        price: U128,
    ) {
        assert_one_yocto();
        let contract_and_token_id = format!("{}{}{}", nft_contract_id, DELIMETER, token_id);
        let mut market_data = self
            .market
            .get(&contract_and_token_id)
            .expect("Token Id does not exist ");

        assert_eq!(
            market_data.owner_id,
            env::predecessor_account_id(),
            "Seller only"
        );

        assert_eq!(
            ft_token_id, market_data.ft_token_id,
            "ft_token_id differs"
        ); // sanity check

        assert!(
            price.0 < NFT_MAX_PRICE,
            "Price higher than {}",
            NFT_MAX_PRICE
        );

        market_data.price = price.into();
        self.market.insert(&contract_and_token_id, &market_data);

        env::log_str(
            &json!({
                "type": "update_market_data",
                "params": {
                    "owner_id": market_data.owner_id,
                    "nft_contract_id": nft_contract_id,
                    "token_id": token_id,
                    "ft_token_id": ft_token_id,
                    "price": price,
                }
            })
            .to_string(),
        );
    }

    fn internal_add_market_data(
        &mut self,
        owner_id: AccountId,
        approval_id: u64,
        nft_contract_id: AccountId,
        token_id: TokenId,
        ft_token_id: AccountId,
        price: U128,
        started_at: Option<U64>,
        ended_at: Option<U64>,
        end_price: Option<U128>,
        is_auction: Option<bool>,
    ) {
        let contract_and_token_id = format!("{}{}{}", nft_contract_id, DELIMETER, token_id);

        let bids: Option<Bids> = match is_auction {
            Some(u) => {
                if u {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            None => None,
        };

        let current_time: u64 = env::block_timestamp();

        if started_at.is_some() {
            assert!(started_at.unwrap().0 >= current_time);

            if ended_at.is_some() {
                assert!(started_at.unwrap().0 < ended_at.unwrap().0);
            }
        }

        if ended_at.is_some() {
            assert!(ended_at.unwrap().0 >= current_time);
        }

        if end_price.is_some() {
            assert!(
                end_price.unwrap().0 < price.0,
                "End price is more than starting price"
            );
        }

        assert!(
            price.0 < NFT_MAX_PRICE,
            "Price higher than {}",
            NFT_MAX_PRICE
        );

        self.market.insert(
            &contract_and_token_id,
            &MarketData {
                owner_id: owner_id.clone().into(),
                approval_id,
                nft_contract_id: nft_contract_id.clone().into(),
                token_id: token_id.clone(),
                ft_token_id: ft_token_id.clone(),
                price: price.into(),
                bids: bids,
                started_at: match started_at {
                    Some(x) => Some(x.0),
                    None => None,
                },
                ended_at: match ended_at {
                    Some(x) => Some(x.0),
                    None => None,
                },
                end_price: match end_price {
                    Some(x) => Some(x.0),
                    None => None,
                },
                accept_nft_contract_id: None,
                accept_token_id: None,
                is_auction: is_auction,
            },
        );

        let mut token_ids = self.by_owner_id.get(&owner_id).unwrap_or_else(|| {
            UnorderedSet::new(
                StorageKey::ByOwnerIdInner {
                    account_id_hash: hash_account_id(&owner_id),
                }
                .try_to_vec()
                .unwrap(),
            )
        });

        token_ids.insert(&contract_and_token_id);

        self.by_owner_id.insert(&owner_id, &token_ids);

        // update offer trade approval_id
        let owner_contract_account_id_token_id =
            make_triple(&nft_contract_id, &owner_id, &token_id);
        let trade_data = self.trades.get(&owner_contract_account_id_token_id);
        if let Some(mut trade_list) = trade_data {
            trade_list.approval_id = approval_id;
            self.trades
                .insert(&owner_contract_account_id_token_id, &trade_list);
        }

        // set market data transaction fee
        let current_transaction_fee = self.calculate_current_transaction_fee();
        self.market_data_transaction_fee
            .transaction_fee
            .insert(&contract_and_token_id, &current_transaction_fee);

        env::log_str(
            &json!({
                "type": "add_market_data",
                "params": {
                    "owner_id": owner_id,
                    "approval_id": approval_id,
                    "nft_contract_id": nft_contract_id,
                    "token_id": token_id,
                    "ft_token_id": ft_token_id,
                    "price": price,
                    "started_at": started_at,
                    "ended_at": ended_at,
                    "end_price": end_price,
                    "is_auction": is_auction,
                    "transaction_fee": current_transaction_fee.to_string(),
                }
            })
            .to_string(),
        );
    }

    fn internal_delete_market_data(
        &mut self,
        nft_contract_id: &AccountId,
        token_id: &TokenId,
    ) -> Option<MarketData> {
        let contract_and_token_id = format!("{}{}{}", &nft_contract_id, DELIMETER, token_id);

        let market_data: Option<MarketData> =
            if let Some(market_data) = self.market.get(&contract_and_token_id) {
                self.market.remove(&contract_and_token_id);

                if let Some(ref bids) = market_data.bids {
                    for bid in bids {
                        Promise::new(bid.bidder_id.clone()).transfer(bid.price.0);
                    }
                };

                Some(market_data)
            } else {
                None
            };

        market_data.map(|market_data| {
            let by_owner_id = self.by_owner_id.get(&market_data.owner_id);
            if let Some(mut by_owner_id) = by_owner_id {
                by_owner_id.remove(&contract_and_token_id);
                if by_owner_id.is_empty() {
                    self.by_owner_id.remove(&market_data.owner_id);
                } else {
                    self.by_owner_id.insert(&market_data.owner_id, &by_owner_id);
                }
            }
            market_data
        })
    }

    #[payable]
    pub fn delete_market_data(&mut self, nft_contract_id: AccountId, token_id: TokenId) {
        assert_one_yocto();
        let contract_and_token_id = format!("{}{}{}", nft_contract_id, DELIMETER, token_id);

        let market_data: Option<MarketData> =
            if let Some(market_data) = self.market.get(&contract_and_token_id) {
                Some(market_data)
            } else {
                None
            };

        let market_data: MarketData = market_data.expect("Market data does not exist");

        assert!(
            [market_data.owner_id.clone(), self.owner_id.clone()]
                .contains(&env::predecessor_account_id()),
            "Seller or owner only"
        );

        self.internal_delete_market_data(&nft_contract_id, &token_id);

        env::log_str(
            &json!({
                "type": "delete_market_data",
                "params": {
                    "owner_id": market_data.owner_id,
                    "nft_contract_id": nft_contract_id,
                    "token_id": token_id,
                }
            })
            .to_string(),
        );
    }

    // Storage

    #[payable]
    pub fn storage_deposit(&mut self, account_id: Option<AccountId>) {
        let storage_account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(env::predecessor_account_id);
        let deposit = env::attached_deposit();
        assert!(
            deposit >= NFT_LISTING_STORAGE_PRICE,
            "Requires minimum deposit of {}",
            NFT_LISTING_STORAGE_PRICE
        );

        let mut balance: u128 = self.storage_deposits.get(&storage_account_id).unwrap_or(0);
        balance += deposit;
        self.storage_deposits.insert(&storage_account_id, &balance);
    }

    #[payable]
    pub fn storage_withdraw(&mut self) {
        assert_one_yocto();
        let owner_id = env::predecessor_account_id();
        let mut amount = self.storage_deposits.remove(&owner_id).unwrap_or(0);
        let market_data_owner = self.by_owner_id.get(&owner_id);
        let len = market_data_owner.map(|s| s.len()).unwrap_or_default();
        let diff = u128::from(len) * NFT_LISTING_STORAGE_PRICE;
        amount -= diff;
        if amount > 0 {
            Promise::new(owner_id.clone()).transfer(amount);
        }
        if diff > 0 {
            self.storage_deposits.insert(&owner_id, &diff);
        }
    }

    pub fn storage_minimum_balance(&self) -> U128 {
        U128(NFT_LISTING_STORAGE_PRICE)
    }

    pub fn storage_balance_of(&self, account_id: AccountId) -> U128 {
        self.storage_deposits.get(&account_id).unwrap_or(0).into()
    }

    // View

    pub fn get_market_data(self, nft_contract_id: AccountId, token_id: TokenId) -> MarketDataJson {
        let contract_and_token_id = format!("{}{}{}", nft_contract_id, DELIMETER, token_id);
        let market_data: Option<MarketData> =
            if let Some(market_data) = self.market.get(&contract_and_token_id) {
                Some(market_data)
            } else {
                None
            };

        let market_data = market_data.expect("Market data does not exist");

        let mut price = market_data.price;

        if market_data.is_auction.is_some() && market_data.end_price.is_some() {
            let current_time = env::block_timestamp();
            let end_price = market_data.end_price.unwrap();
            let started_at = market_data.started_at.unwrap();
            let ended_at = market_data.ended_at.unwrap();

            if current_time < started_at {
                // Use current market_data.price
            } else if current_time > ended_at {
                price = end_price;
            } else {
                let time_since_start = current_time - started_at;
                let duration = ended_at - started_at;
                price = price - ((price - end_price) / duration as u128) * time_since_start as u128;
            }
        }

        let current_transaction_fee = self
            .get_market_data_transaction_fee(&market_data.nft_contract_id, &market_data.token_id);

        MarketDataJson {
            owner_id: market_data.owner_id,
            approval_id: market_data.approval_id.into(),
            nft_contract_id: market_data.nft_contract_id,
            token_id: market_data.token_id,
            ft_token_id: market_data.ft_token_id, // "near" for NEAR token
            price: price.into(),
            bids: market_data.bids,
            started_at: market_data.started_at.map(|x| x.into()),
            ended_at: market_data.ended_at.map(|x| x.into()),
            end_price: market_data.end_price.map(|x| x.into()),
            is_auction: market_data.is_auction,
            transaction_fee: current_transaction_fee.into(),
        }
    }

    pub fn approved_ft_token_ids(&self) -> Vec<AccountId> {
        self.approved_ft_token_ids.to_vec()
    }

    pub fn approved_nft_contract_ids(&self) -> Vec<AccountId> {
        self.approved_nft_contract_ids.to_vec()
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    pub fn get_treasury(&self) -> AccountId {
        self.treasury_id.clone()
    }

    pub fn get_supply_by_owner_id(&self, account_id: AccountId) -> U64 {
        self.by_owner_id
            .get(&account_id)
            .map_or(0, |by_owner_id| by_owner_id.len())
            .into()
    }

    // private fn

    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Owner only"
        )
    }
}

pub fn hash_account_id(account_id: &AccountId) -> CryptoHash {
    let mut hash = CryptoHash::default();
    hash.copy_from_slice(&env::sha256(account_id.as_bytes()));
    hash
}

pub fn hash_contract_account_id_token_id(
    contract_account_id_token_id: &ContractAccountIdTokenId,
) -> CryptoHash {
    let mut hash = CryptoHash::default();
    hash.copy_from_slice(&env::sha256(contract_account_id_token_id.as_bytes()));
    hash
}

pub fn to_sec(timestamp: Timestamp) -> TimestampSec {
    (timestamp / 10u64.pow(9)) as u32
}

#[ext_contract(ext_self)]
trait ExtSelf {
    fn resolve_purchase(
        &mut self,
        buyer_id: AccountId,
        market_data: MarketData,
        price: U128,
    ) -> Promise;

    fn resolve_offer(
        &mut self,
        seller_id: AccountId,
        offer_data: OfferData,
        token_id: TokenId,
    ) -> Promise;

    fn callback_first_trade(
        &mut self,
        seller_nft_contract_id: AccountId,
        seller_token_id: TokenId,
        seller_approval_id: u64,
    ) -> Promise;

    fn callback_second_trade(
        &mut self,
        buyer_id: AccountId,
        buyer_nft_contract_id: AccountId,
        buyer_token_id: TokenId,
        seller_id: AccountId,
        seller_nft_contract_id: AccountId,
        seller_token_id: TokenId,
    ) -> Promise;
}

fn add_accounts(accounts: Option<Vec<AccountId>>, set: &mut UnorderedSet<AccountId>) {
    accounts.map(|ids| {
        ids.iter().for_each(|id| {
            set.insert(id);
        })
    });
}

fn remove_accounts(accounts: Option<Vec<AccountId>>, set: &mut UnorderedSet<AccountId>) {
    accounts.map(|ids| {
        ids.iter().for_each(|id| {
            set.remove(id);
        })
    });
}

fn make_triple(nft_contract_id: &AccountId, buyer_id: &AccountId, token: &str) -> String {
    format!(
        "{}{}{}{}{}",
        nft_contract_id, DELIMETER, buyer_id, DELIMETER, token
    )
}

fn make_key_owner_by_id_trade(contract_account_id_token_id: String) -> String {
    format!("{}{}trade", contract_account_id_token_id, DELIMETER)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Contract::new(
            accounts(0),
            accounts(1),
            None,
            Some(vec![accounts(2)]),
            Some(vec![accounts(2)]),
            500,
        );
        (context, contract)
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let contract = Contract::new(
            accounts(0),
            accounts(1),
            None,
            Some(vec![accounts(2)]),
            Some(vec![accounts(2)]),
            500,
        );
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.get_owner(), accounts(0));
        assert_eq!(contract.get_treasury(), accounts(1));
        assert_eq!(contract.approved_ft_token_ids(), vec![near_account()]);
        assert_eq!(contract.approved_nft_contract_ids(), vec![accounts(2)]);
        assert_eq!(contract.transaction_fee.current_fee, 500);
    }

    #[test]
    fn test_set_treasury() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.set_treasury(accounts(5));
        let new_treasury: AccountId = contract.get_treasury();
        assert_eq!(new_treasury, accounts(5));
    }

    #[test]
    #[should_panic(expected = "Owner only")]
    fn test_invalid_set_treasury() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(1)
            .build());

        contract.set_treasury(accounts(5));
    }

    #[test]
    fn test_change_owner() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.change_owner(accounts(5));
        let new_owner: AccountId = contract.get_owner();
        assert_eq!(new_owner, accounts(5));
    }

    #[test]
    #[should_panic(expected = "Owner only")]
    fn test_invalid_change_owner() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(5))
            .attached_deposit(1)
            .build());

        contract.change_owner(accounts(5));
    }

    #[test]
    fn test_add_approved_ft_token_ids() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.add_approved_ft_token_ids(vec![accounts(5)]);
        let approved_fts = contract.approved_ft_token_ids();
        assert_eq!(approved_fts, vec![near_account(), accounts(5)]);
    }

    #[test]
    fn test_add_approved_nft_contract_ids() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.add_approved_nft_contract_ids(vec![accounts(5)]);
        let approved_nfts = contract.approved_nft_contract_ids();
        assert_eq!(approved_nfts, vec![accounts(2), accounts(5)]);
    }

    #[test]
    fn test_remove_approved_nft_contract_ids() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.add_approved_nft_contract_ids(vec![accounts(5)]);
        contract.remove_approved_nft_contract_ids(vec![accounts(5)]);
        let approved_nfts = contract.approved_nft_contract_ids();
        assert_eq!(approved_nfts, vec![accounts(2)]);
    }

    #[test]
    fn test_internal_add_market_data() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context.predecessor_account_id(accounts(0)).build());

        contract.internal_add_market_data(
            accounts(3),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1 * 10u128.pow(24)),
            None,
            None,
            None,
            None,
        );

        let market = contract.get_market_data(accounts(2), "1:1".to_string());
        assert_eq!(market.owner_id, accounts(3));
        assert_eq!(market.approval_id, U64::from(1));
        assert_eq!(market.ft_token_id, near_account());
        assert_eq!(market.nft_contract_id, accounts(2));
        assert_eq!(market.owner_id, accounts(3));
        assert_eq!(market.token_id, "1:1".to_string());
        assert_eq!(market.price, U128::from(1 * 10u128.pow(24)));
    }

    #[test]
    #[should_panic(expected = "Price higher than 1000000000000000000000000000000000")]
    fn test_invalid_price_higher_than_max_price() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context.predecessor_account_id(accounts(0)).build());

        contract.internal_add_market_data(
            accounts(3),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1_000_000_000 * 10u128.pow(24)),
            None,
            None,
            None,
            None,
        );
    }

    #[test]
    #[should_panic(expected = "Price higher than 1000000000000000000000000000000000")]
    fn test_invalid_price_higher_than_max_price_update() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context.predecessor_account_id(accounts(0)).build());

        contract.internal_add_market_data(
            accounts(0),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1 * 10u128.pow(24)),
            None,
            None,
            None,
            None,
        );

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.update_market_data(
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1_000_000_000 * 10u128.pow(24)),
        );
    }

    #[test]
    #[should_panic(expected = "Seller only")]
    fn test_invalid_update_market_data() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context.predecessor_account_id(accounts(0)).build());

        contract.internal_add_market_data(
            accounts(3),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1 * 10u128.pow(24)),
            None,
            None,
            None,
            None,
        );

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.update_market_data(
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(2 * 10u128.pow(24)),
        );
    }

    #[test]
    fn test_update_market_data() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context.predecessor_account_id(accounts(0)).build());

        contract.internal_add_market_data(
            accounts(3),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1 * 10u128.pow(24)),
            None,
            None,
            None,
            None,
        );

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());

        contract.update_market_data(
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(2 * 10u128.pow(24)),
        );

        let market = contract.get_market_data(accounts(2), "1:1".to_string());
        assert_eq!(market.price, U128::from(2 * 10u128.pow(24)));
    }

    #[test]
    #[should_panic(expected = "Market data does not exist")]
    fn test_delete_market_data() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context.predecessor_account_id(accounts(0)).build());

        contract.internal_add_market_data(
            accounts(3),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1 * 10u128.pow(24)),
            None,
            None,
            None,
            None,
        );

        testing_env!(context
            .predecessor_account_id(accounts(3))
            .attached_deposit(1)
            .build());

        contract.delete_market_data(accounts(2), "1:1".to_string());

        contract.get_market_data(accounts(2), "1:1".to_string());
    }

    #[test]
    fn test_storage_deposit() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(NFT_LISTING_STORAGE_PRICE)
            .build());

        contract.storage_deposit(None);

        let storage_balance = contract.storage_balance_of(accounts(0)).0;
        assert_eq!(NFT_LISTING_STORAGE_PRICE, storage_balance);

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.storage_withdraw();

        let storage_balance = contract.storage_balance_of(accounts(0)).0;
        assert_eq!(0, storage_balance);
    }

    #[test]
    fn test_add_offer() {
        let (mut context, mut contract) = setup_contract();

        let one_near = 10u128.pow(24);

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(one_near)
            .build());

        contract.internal_add_offer(
            accounts(3),
            Some("1:1".to_string()),
            None,
            near_account(),
            U128(one_near),
            accounts(0),
        );

        let offer_data =
            contract.get_offer(accounts(3), accounts(0), Some("1:1".to_string()), None);

        assert_eq!(offer_data.buyer_id, accounts(0));
        assert_eq!(offer_data.price, U128(one_near));
    }

    #[test]
    #[should_panic(expected = "Offer does not exist")]
    fn test_delete_offer() {
        let (mut context, mut contract) = setup_contract();

        let one_near = 10u128.pow(24);

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(one_near)
            .build());

        contract.internal_add_offer(
            accounts(3),
            Some("1:1".to_string()),
            None,
            near_account(),
            U128(one_near),
            accounts(0),
        );

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.delete_offer(accounts(3), Some("1:1".to_string()), None);

        contract.get_offer(accounts(3), accounts(1), Some("1:1".to_string()), None);
    }

    #[test]
    fn test_add_trade() {
        let (mut context, mut contract) = setup_contract();

        let one_near = 10u128.pow(24);

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(one_near)
            .build());

        contract.internal_add_trade(
            accounts(3),
            Some("1:1".to_string()),
            None,
            accounts(1),
            Some("1:2".to_string()),
            accounts(2),
            1,
        );

        let trade_data = contract.get_trade(
            accounts(3),
            Some("1:1".to_string()),
            None,
            accounts(2),
            accounts(1),
            "1:2".to_string(),
        );

        assert_eq!(trade_data.token_id.unwrap().to_string(), "1:1");
        assert_eq!(trade_data.nft_contract_id, accounts(3));
    }

    #[test]
    #[should_panic(expected = "Trade list does not exist")]
    fn test_delete_trade() {
        let (mut context, mut contract) = setup_contract();

        let one_near = 10u128.pow(24);

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(one_near)
            .build());

        contract.internal_add_trade(
            accounts(3),
            Some("1:1".to_string()),
            None,
            accounts(1),
            Some("1:1".to_string()),
            accounts(2),
            1,
        );

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.delete_trade(
            accounts(3),
            Some("1:1".to_string()),
            None,
            accounts(1),
            "1:2".to_string(),
        );
        contract.get_trade(
            accounts(3),
            Some("1:1".to_string()),
            None,
            accounts(1),
            accounts(1),
            "1:2".to_string(),
        );
    }

    #[test]
    fn test_internal_add_market_data_auction() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context.predecessor_account_id(accounts(0)).build());

        contract.internal_add_market_data(
            accounts(3),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1 * 10u128.pow(24)),
            None,
            None,
            None,
            Some(true),
        );

        let market = contract.get_market_data(accounts(2), "1:1".to_string());
        assert_eq!(market.is_auction, Some(true));
    }

    #[test]
    #[should_panic(expected = "The auction has not ended yet.")]
    fn test_bid_invalid_purchase() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context.predecessor_account_id(accounts(0)).build());

        contract.internal_add_market_data(
            accounts(3),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1 * 10u128.pow(24)),
            None,
            None,
            None,
            Some(true),
        );

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(10u128.pow(24))
            .build());

        contract.buy(accounts(2), "1:1".to_string(), None, None);
    }

    #[test]
    fn test_add_bid_and_accept() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context.predecessor_account_id(accounts(0)).build());

        contract.internal_add_market_data(
            accounts(0),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1 * 10u128.pow(24)),
            None,
            None,
            None,
            Some(true),
        );

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .attached_deposit(10u128.pow(24) + 1)
            .build());

        contract.add_bid(
            accounts(2),
            near_account(),
            "1:1".to_string(),
            U128::from(10u128.pow(24) + 1),
        );

        testing_env!(context
            .predecessor_account_id(accounts(4))
            .attached_deposit(10u128.pow(24) + 2)
            .build());

        contract.add_bid(
            accounts(2),
            near_account(),
            "1:1".to_string(),
            U128::from(10u128.pow(24) + 2),
        );

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.accept_bid(accounts(2), "1:1".to_string());
    }

    #[test]
    fn test_change_transaction_fee_immediately() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        contract.set_tx_fee(100, None);

        assert_eq!(contract.get_transaction_fee().current_fee, 100);
    }

    #[test]
    fn test_change_transaction_fee_with_time() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        assert_eq!(contract.get_transaction_fee().current_fee, 500);
        assert_eq!(contract.get_transaction_fee().next_fee, None);
        assert_eq!(contract.get_transaction_fee().start_time, None);

        let next_fee: u16 = 100;
        let start_time: Timestamp = 1618109122863866400;
        let start_time_sec: TimestampSec = to_sec(start_time);
        contract.set_tx_fee(next_fee, Some(start_time_sec));

        assert_eq!(contract.get_transaction_fee().current_fee, 500);
        assert_eq!(contract.get_transaction_fee().next_fee, Some(next_fee));
        assert_eq!(
            contract.get_transaction_fee().start_time,
            Some(start_time_sec)
        );

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .block_timestamp(start_time + 1)
            .build());

        contract.calculate_current_transaction_fee();
        assert_eq!(contract.get_transaction_fee().current_fee, next_fee);
        assert_eq!(contract.get_transaction_fee().next_fee, None);
        assert_eq!(contract.get_transaction_fee().start_time, None);
    }
    #[test]
    fn test_transaction_fee_locked() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .predecessor_account_id(accounts(0))
            .attached_deposit(1)
            .build());

        assert_eq!(contract.get_transaction_fee().current_fee, 500);
        assert_eq!(contract.get_transaction_fee().next_fee, None);
        assert_eq!(contract.get_transaction_fee().start_time, None);

        let next_fee: u16 = 100;
        let start_time: Timestamp = 1618109122863866400;
        let start_time_sec: TimestampSec = to_sec(start_time);
        contract.set_tx_fee(next_fee, Some(start_time_sec));

        contract.internal_add_market_data(
            accounts(3),
            1,
            accounts(2),
            "1:1".to_string(),
            near_account(),
            U128::from(1 * 10u128.pow(24)),
            None,
            None,
            None,
            None,
        );

        assert_eq!(contract.get_transaction_fee().current_fee, 500);
        assert_eq!(contract.get_transaction_fee().next_fee, Some(next_fee));
        assert_eq!(
            contract.get_transaction_fee().start_time,
            Some(start_time_sec)
        );

        testing_env!(context
            .predecessor_account_id(accounts(1))
            .block_timestamp(start_time + 1)
            .build());

        contract.calculate_current_transaction_fee();
        assert_eq!(contract.get_transaction_fee().current_fee, next_fee);
        assert_eq!(contract.get_transaction_fee().next_fee, None);
        assert_eq!(contract.get_transaction_fee().start_time, None);

        let market = contract.get_market_data(accounts(2), "1:1".to_string());
        let market_data_transaction_fee: u128 = market.transaction_fee.into();
        assert_eq!(market_data_transaction_fee, 500);
    }
}
