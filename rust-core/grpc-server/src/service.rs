//! `SwapService` gRPC implementation.
//!
//! Every RPC delegates to one of the four core traits (or the orchestrator
//! for the atomic swap flow). The service holds `Arc<dyn Trait>` handles so
//! mocks can be injected for tests and real implementations swapped in for
//! production.

use crate::convert;
use crate::proto::swap_service_server::SwapService;
use crate::proto::*;
use common::{Amount, AssetClass, AssetId, Chain, TxIntent, TxStatus};
use orchestrator::{Dependencies, SwapRequest};
use rust_decimal::Decimal;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct SwapServiceImpl {
    pub deps: Arc<Dependencies>,
    // Cache: quote_id (String) -> Quote
    pub quote_cache: Arc<RwLock<HashMap<String, transaction_engine::types::Quote>>>,
}


impl SwapServiceImpl {
    pub fn new(deps: Arc<Dependencies>) -> Self {
        Self { deps,
	       quote_cache: Arc::new(RwLock::new(HashMap::new())),
 }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn status_internal<E: std::fmt::Display>(e: E) -> Status {
    Status::internal(format!("{e}"))
}

fn status_invalid<E: std::fmt::Display>(e: E) -> Status {
    Status::invalid_argument(format!("{e}"))
}

fn status_not_found<E: std::fmt::Display>(e: E) -> Status {
    Status::not_found(format!("{e}"))
}

fn chain_from_proto(s: &str) -> Result<Chain, Status> {
    convert::chain_from_str(s)
        .ok_or_else(|| Status::invalid_argument(format!("unknown chain: {s}")))
}

fn parse_intent(
    user_id: &str,
    from_asset: &str,
    to_asset: &str,
    amount: &str,
    slippage_bps: u32,
) -> Result<TxIntent, Status> {
    let user = convert::parse_uuid(user_id).map_err(status_invalid)?;
    let from = symbol_to_asset(from_asset).ok_or_else(|| {
        Status::invalid_argument(format!("unknown from_asset symbol: {from_asset}"))
    })?;
    let to = symbol_to_asset(to_asset)
        .ok_or_else(|| Status::invalid_argument(format!("unknown to_asset symbol: {to_asset}")))?;
    let value = convert::parse_decimal(amount).map_err(status_invalid)?;
    Ok(TxIntent {
        intent_id: Uuid::new_v4(),
        user_id: user,
        from: from.clone(),
        to,
        amount: Amount {
            asset: from,
            value,
        },
        slippage_bps: u16::try_from(slippage_bps)
            .map_err(|_| Status::invalid_argument("slippage_bps too large"))?,
        deadline: chrono::Utc::now() + chrono::Duration::minutes(5),
    })
}

fn symbol_to_asset(symbol: &str) -> Option<AssetId> {
    let s = symbol.to_ascii_uppercase();
    let (chain, decimals, class) = match s.as_str() {
        "ETH" => (Chain::Ethereum, 18, AssetClass::NativeToken),
        "USDC" => (Chain::Ethereum, 6, AssetClass::Erc20),
        "USDT" => (Chain::Ethereum, 6, AssetClass::Erc20),
        "MATIC" | "POL" => (Chain::Polygon, 18, AssetClass::NativeToken),
        "ARB" => (Chain::Arbitrum, 18, AssetClass::NativeToken),
        "SOL" => (Chain::Solana, 9, AssetClass::NativeToken),
        _ => return None,
    };
    Some(AssetId {
        chain,
        contract: None,
        symbol: s,
        decimals,
        class,
    })
}

fn tx_status_to_str(status: &TxStatus) -> (&'static str, u64, u32, String) {
    match status {
        TxStatus::Pending => ("pending", 0, 0, String::new()),
        TxStatus::Confirmed { block, confirmations } => {
            ("confirmed", *block, *confirmations, String::new())
        }
        TxStatus::Failed { reason } => ("failed", 0, 0, reason.clone()),
    }
}

// ---------------------------------------------------------------------------
// SwapService impl
// ---------------------------------------------------------------------------

#[tonic::async_trait]
impl SwapService for SwapServiceImpl {
        async fn quote(
        &self,
        request: Request<QuoteRequest>,
    ) -> Result<Response<QuoteResponse>, Status> {
        let req = request.into_inner();
        
        let intent = parse_intent(
            &req.user_id,
            &req.from_asset,
            &req.to_asset,
            &req.amount,
            req.slippage_bps,
        )?;

        // 1. Generate the quote using the engine
        let quote = self.deps.tx_engine.quote(intent).await.map_err(status_internal)?;

        // 2. Store it in the cache for the Build step
        self.quote_cache
            .write()
            .await
            .insert(quote.quote_id.to_string(), quote.clone());

        // 3. Return the response
        Ok(Response::new(QuoteResponse {
            quote_id: quote.quote_id.to_string(),
            expected_out: quote.expected_out.value.to_string(),
            minimum_out: quote.minimum_out.value.to_string(),
            estimated_fee: quote.estimated_fee.value.to_string(),
            expires_at: quote.expires_at.to_rfc3339(),
            route_summary: format!("{} hops", quote.route.len()),
        }))
    }
     

     async fn build(
        &self,
        request: Request<BuildRequest>,
    ) -> Result<Response<BuildResponse>, Status> {
        let req = request.into_inner();
        
        // 1. Look up the quote in the cache
        let cache = self.quote_cache.read().await;
        let quote = cache.get(&req.quote_id)
            .ok_or_else(|| Status::not_found(format!("Quote {} not found", req.quote_id)))?;

        // 2. Call the engine's build method
        let unsigned_tx = self.deps.tx_engine.build(quote).await.map_err(status_internal)?;

        // 3. Return the JSON string
        Ok(Response::new(BuildResponse {
            unsigned_tx_json: String::from_utf8_lossy(&unsigned_tx).to_string(),
        }))
    }

    async fn simulate(
        &self,
        request: Request<SimulateRequest>,
    ) -> Result<Response<SimulateResponse>, Status> {
        let req = request.into_inner();
        
        // 1. Convert the JSON string back to bytes
        let unsigned_tx = req.unsigned_tx_json.into_bytes();

        // 2. Call the engine's simulate method
        let result = self.deps.tx_engine.simulate(common::Chain::Ethereum, &unsigned_tx).await.map_err(status_internal)?;

        // 3. Return the result
        Ok(Response::new(SimulateResponse {
            success: result.success,
            revert_reason: result.revert_reason.unwrap_or_default(),
            gas_used: result.gas_used,
        }))
    }

    async fn execute_swap(
        &self,
        request: Request<ExecuteSwapRequest>,
    ) -> Result<Response<ExecuteSwapResponse>, Status> {
        let r = request.into_inner();
        let from_account =
            ledger::AccountId(convert::parse_uuid(&r.from_account).map_err(status_invalid)?);
        let to_account =
            ledger::AccountId(convert::parse_uuid(&r.to_account).map_err(status_invalid)?);
        let wallet_handle = wallet_service::PublicKeyHandle(
            convert::parse_uuid(&r.wallet_handle).map_err(status_invalid)?,
        );

        let user_id = convert::parse_uuid(&r.user_id).map_err(status_invalid)?;
        let _quote_id = convert::parse_uuid(&r.quote_id).map_err(status_invalid)?;

        // Placeholder intent — production would look up by quote_id.
        let intent = TxIntent {
            intent_id: Uuid::new_v4(),
            user_id,
            from: symbol_to_asset("ETH").unwrap(),
            to: symbol_to_asset("USDC").unwrap(),
            amount: Amount {
                asset: symbol_to_asset("ETH").unwrap(),
                value: Decimal::new(1, 0),
            },
            slippage_bps: 50,
            deadline: chrono::Utc::now() + chrono::Duration::minutes(5),
        };

        let req = SwapRequest {
            from_account,
            to_account,
            wallet_handle,
            signature: wallet_service::Signature(r.signature),
            intent,
        };

        let receipt = orchestrator::execute_swap(&self.deps, req)
            .await
            .map_err(status_internal)?;

        Ok(Response::new(ExecuteSwapResponse {
            tx_hash: receipt.tx_hash,
            chain: convert::chain_to_str(receipt.chain),
            ledger_post_id: receipt.ledger_post_id.to_string(),
            settled_at: receipt.settled_at.to_rfc3339(),
        }))
    }

    async fn get_swap_status(
        &self,
        request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let r = request.into_inner();
        let chain = chain_from_proto(&r.chain)?;
        let result = self
            .deps
            .tx_engine
            .status(chain, &r.tx_hash)
            .await
            .map_err(status_internal)?;

        let (status_str, block, confirmations, error) = tx_status_to_str(&result.status);

        Ok(Response::new(StatusResponse {
            tx_hash: r.tx_hash,
            status: status_str.into(),
            block,
            confirmations,
            error,
        }))
    }

    async fn register_wallet(
        &self,
        request: Request<RegisterWalletRequest>,
    ) -> Result<Response<RegisterWalletResponse>, Status> {
        let r = request.into_inner();
        let address = wallet_service::Address(r.address);
        let signature = wallet_service::Signature(r.signature);

        let info = self
            .deps
            .wallet
            .register_address(r.chain_id, address, &r.challenge, &signature)
            .await
            .map_err(status_internal)?;

        Ok(Response::new(RegisterWalletResponse {
            handle: info.handle.0.to_string(),
            address: info.address.0,
            public_key: hex::encode(&info.public_key.0),
            chain_id: info.chain_id,
            registered_at: info.registered_at.to_rfc3339(),
        }))
    }

    async fn get_wallet(
        &self,
        request: Request<GetWalletRequest>,
    ) -> Result<Response<GetWalletResponse>, Status> {
        let r = request.into_inner();
        let handle = wallet_service::PublicKeyHandle(
            convert::parse_uuid(&r.handle).map_err(status_invalid)?,
        );
        let info = self
            .deps
            .wallet
            .lookup_address(&handle)
            .await
            .map_err(status_not_found)?;

        Ok(Response::new(GetWalletResponse {
            handle: info.handle.0.to_string(),
            address: info.address.0,
            public_key: hex::encode(&info.public_key.0),
            chain_id: info.chain_id,
            origin: format!("{:?}", info.origin).to_ascii_lowercase(),
            registered_at: info.registered_at.to_rfc3339(),
        }))
    }

    async fn list_wallets(
        &self,
        request: Request<ListWalletsRequest>,
    ) -> Result<Response<ListWalletsResponse>, Status> {
        let r = request.into_inner();
        let infos = self
            .deps
            .wallet
            .list_addresses(&r.user_id)
            .await
            .map_err(status_internal)?;

        let wallets = infos
            .into_iter()
            .map(|info| GetWalletResponse {
                handle: info.handle.0.to_string(),
                address: info.address.0,
                public_key: hex::encode(&info.public_key.0),
                chain_id: info.chain_id,
                origin: format!("{:?}", info.origin).to_ascii_lowercase(),
                registered_at: info.registered_at.to_rfc3339(),
            })
            .collect();

        Ok(Response::new(ListWalletsResponse { wallets }))
    }

    async fn verify_signature(
        &self,
        request: Request<VerifySignatureRequest>,
    ) -> Result<Response<VerifySignatureResponse>, Status> {
        let r = request.into_inner();
        let address = wallet_service::Address(r.address);
        let sig = wallet_service::Signature(r.signature);

        match self
            .deps
            .wallet
            .verify_signature(&address, &r.payload, &sig)
            .await
        {
            Ok(()) => Ok(Response::new(VerifySignatureResponse {
                valid: true,
                recovered: address.0,
            })),
            Err(_) => Ok(Response::new(VerifySignatureResponse {
                valid: false,
                recovered: String::new(),
            })),
        }
    }

    async fn get_portfolio(
        &self,
        request: Request<PortfolioRequest>,
    ) -> Result<Response<PortfolioResponse>, Status> {
        let r = request.into_inner();
        let account_id =
            ledger::AccountId(convert::parse_uuid(&r.account_id).map_err(status_invalid)?);
        let balances = self
            .deps
            .ledger
            .balances(&account_id)
            .await
            .map_err(status_internal)?;

        let positions = balances
            .into_iter()
            .map(|b| Position {
                asset: Some(convert::domain_asset_to_proto(&b.asset)),
                amount: b.amount.to_string(),
                value_usd: b.amount.to_string(),
                change_24h_pct: "0".into(),
            })
            .collect();

        Ok(Response::new(PortfolioResponse {
            total_value_usd: "0".into(),
            positions,
        }))
    }

    async fn list_transactions(
        &self,
        request: Request<TransactionsRequest>,
    ) -> Result<Response<TransactionsResponse>, Status> {
        let _ = request.into_inner();
        Ok(Response::new(TransactionsResponse {
            transactions: vec![],
        }))
    }

    async fn get_limits(
        &self,
        request: Request<LimitsRequest>,
    ) -> Result<Response<LimitsResponse>, Status> {
        let r = request.into_inner();
        let user = convert::parse_uuid(&r.user_id).map_err(status_invalid)?;
        let limits = self
            .deps
            .policy
            .get_limits(user)
            .await
            .map_err(status_internal)?;

        Ok(Response::new(LimitsResponse {
            per_tx_usd: limits.per_tx_usd.to_string(),
            daily_withdrawal_usd: limits.daily_withdrawal_usd.to_string(),
            velocity_per_hour: limits.velocity_per_hour,
        }))
    }

    async fn daily_total(
        &self,
        request: Request<DailyTotalRequest>,
    ) -> Result<Response<DailyTotalResponse>, Status> {
        let r = request.into_inner();
        let user = convert::parse_uuid(&r.user_id).map_err(status_invalid)?;
        let total = self
            .deps
            .policy
            .daily_total_usd(user)
            .await
            .map_err(status_internal)?;

        Ok(Response::new(DailyTotalResponse {
            total_usd: total.to_string(),
        }))
    }
}
