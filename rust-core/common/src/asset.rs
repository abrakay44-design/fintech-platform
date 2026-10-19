use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Chain { Solana, Ethereum, Polygon, Arbitrum, Cosmos, Near }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AssetClass { NativeToken, Erc20, Spl, TokenizedEtf, TokenizedStock }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetId {
    pub chain: Chain,
    pub contract: Option<String>,
    pub symbol: String,
    pub decimals: u8,
    pub class: AssetClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    pub asset: AssetId,
    pub value: Decimal,
}
