//! Proto ↔ domain type conversions.

use crate::proto;
use common::{Amount, AssetClass, AssetId, Chain};
use rust_decimal::Decimal;
use std::str::FromStr;

pub fn chain_from_str(s: &str) -> Option<Chain> {
    match s.to_ascii_lowercase().as_str() {
        "solana" => Some(Chain::Solana),
        "ethereum" | "eth" => Some(Chain::Ethereum),
        "polygon" | "matic" => Some(Chain::Polygon),
        "arbitrum" | "arb" => Some(Chain::Arbitrum),
        "cosmos" | "atom" => Some(Chain::Cosmos),
        "near" => Some(Chain::Near),
        _ => None,
    }
}

pub fn chain_to_str(chain: Chain) -> String {
    match chain {
        Chain::Solana => "solana",
        Chain::Ethereum => "ethereum",
        Chain::Polygon => "polygon",
        Chain::Arbitrum => "arbitrum",
        Chain::Cosmos => "cosmos",
        Chain::Near => "near",
    }
    .into()
}

pub fn asset_class_from_str(s: &str) -> AssetClass {
    match s.to_ascii_lowercase().as_str() {
        "native_token" => AssetClass::NativeToken,
        "erc20" => AssetClass::Erc20,
        "spl" => AssetClass::Spl,
        "tokenized_etf" => AssetClass::TokenizedEtf,
        "tokenized_stock" => AssetClass::TokenizedStock,
        _ => AssetClass::NativeToken,
    }
}

pub fn asset_class_to_str(c: AssetClass) -> String {
    match c {
        AssetClass::NativeToken => "native_token",
        AssetClass::Erc20 => "erc20",
        AssetClass::Spl => "spl",
        AssetClass::TokenizedEtf => "tokenized_etf",
        AssetClass::TokenizedStock => "tokenized_stock",
    }
    .into()
}

pub fn proto_asset_to_domain(a: &proto::AssetId) -> Option<AssetId> {
    let chain = chain_from_str(&a.chain)?;
    Some(AssetId {
        chain,
        contract: if a.contract.is_empty() {
            None
        } else {
            Some(a.contract.clone())
        },
        symbol: a.symbol.clone(),
        decimals: a.decimals as u8,
        class: asset_class_from_str(&a.asset_class),
    })
}

pub fn domain_asset_to_proto(a: &AssetId) -> proto::AssetId {
    proto::AssetId {
        chain: chain_to_str(a.chain),
        contract: a.contract.clone().unwrap_or_default(),
        symbol: a.symbol.clone(),
        decimals: a.decimals as u32,
        asset_class: asset_class_to_str(a.class),
    }
}

pub fn parse_decimal(s: &str) -> Result<Decimal, String> {
    Decimal::from_str(s).map_err(|e| format!("invalid decimal '{s}': {e}"))
}

pub fn amount_to_string(d: Decimal) -> String {
    d.to_string()
}

pub fn parse_uuid(s: &str) -> Result<uuid::Uuid, String> {
    uuid::Uuid::parse_str(s).map_err(|e| format!("invalid uuid '{s}': {e}"))
}

pub fn parse_uuid_handle(s: &str) -> Result<wallet_service::PublicKeyHandle, String> {
    let id = parse_uuid(s)?;
    Ok(wallet_service::PublicKeyHandle(id))
}

pub fn epoch_to_rfc3339(secs: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(secs, 0)
        .single()
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".into())
}

#[allow(dead_code)]
pub fn amount_to_proto(amount: &Amount) -> proto::AssetId {
    domain_asset_to_proto(&amount.asset)
}
