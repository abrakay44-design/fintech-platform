//! Integration tests for the policy engine.

use chrono::{Duration, Utc};
use common::{Amount, AssetClass, AssetId, Chain, TxIntent};
use policy_engine::{Limits, MockPolicyEngine, PolicyDecision, PolicyEngine, ScreenResult, TxRecord};
use rust_decimal::Decimal;
use uuid::Uuid;

fn usdc() -> AssetId {
    AssetId {
        chain: Chain::Ethereum,
        contract: Some("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string()),
        symbol: "USDC".to_string(),
        decimals: 6,
        class: AssetClass::Erc20,
    }
}

fn make_intent(user_id: Uuid, value: Decimal) -> TxIntent {
    TxIntent {
        intent_id: Uuid::new_v4(),
        user_id,
        from: usdc(),
        to: usdc(),
        amount: Amount {
            asset: usdc(),
            value,
        },
        slippage_bps: 50,
        deadline: Utc::now() + Duration::minutes(5),
    }
}

#[tokio::test]
async fn small_transaction_is_allowed() {
    let engine = MockPolicyEngine::new();
    let user = Uuid::new_v4();
    let intent = make_intent(user, Decimal::new(100, 0));

    let decision = engine.evaluate(&intent).await.unwrap();
    assert!(matches!(decision, PolicyDecision::Allow));
}

#[tokio::test]
async fn large_transaction_requires_step_up() {
    let engine = MockPolicyEngine::new();
    let user = Uuid::new_v4();
    let intent = make_intent(user, Decimal::new(5_000, 0));

    let decision = engine.evaluate(&intent).await.unwrap();
    match decision {
        PolicyDecision::RequireStepUp { challenge } => {
            assert!(challenge.expires_at > Utc::now());
            assert!(!challenge.reason.is_empty());
        }
        other => panic!("expected RequireStepUp, got {other:?}"),
    }
}

#[tokio::test]
async fn above_per_tx_limit_is_denied() {
    let engine = MockPolicyEngine::new();
    let user = Uuid::new_v4();
    let intent = make_intent(user, Decimal::new(20_000, 0));

    let result = engine.evaluate(&intent).await;
    assert!(matches!(
        result,
        Err(policy_engine::PolicyError::PerTxLimitExceeded { .. })
    ));
}

#[tokio::test]
async fn zero_amount_is_rejected() {
    let engine = MockPolicyEngine::new();
    let user = Uuid::new_v4();
    let intent = make_intent(user, Decimal::ZERO);

    let result = engine.evaluate(&intent).await;
    assert!(matches!(
        result,
        Err(policy_engine::PolicyError::InvalidAmount { .. })
    ));
}

#[tokio::test]
async fn daily_limit_is_enforced() {
    let engine = MockPolicyEngine::new();
    let user = Uuid::new_v4();

    engine
        .record(TxRecord {
            tx_id: Uuid::new_v4(),
            user_id: user,
            intent: make_intent(user, Decimal::new(9_000, 0)),
            usd_value: Decimal::new(9_000, 0),
            timestamp: Utc::now(),
        })
        .await
        .unwrap();

    engine
        .set_limits(user, Limits {
            per_tx_usd: Decimal::new(10_000, 0),
            daily_withdrawal_usd: Decimal::new(10_000, 0),
            velocity_per_hour: 20,
        })
        .await
        .unwrap();

    let denied_intent = make_intent(user, Decimal::new(5_000, 0));
    let result = engine.evaluate(&denied_intent).await;
    assert!(matches!(
        result,
        Err(policy_engine::PolicyError::DailyLimitExceeded { .. })
    ));
}

#[tokio::test]
async fn sanctioned_address_is_denied() {
    let engine = MockPolicyEngine::new();
    let user = Uuid::new_v4();
    let mut intent = make_intent(user, Decimal::new(100, 0));
    intent.to.contract = Some("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string());

    let result = engine.evaluate(&intent).await;
    assert!(matches!(
        result,
        Err(policy_engine::PolicyError::SanctionedAddress { .. })
    ));
}

#[tokio::test]
async fn screen_address_detects_sanctions() {
    let engine = MockPolicyEngine::new();

    let clean = engine
        .screen_address("0x1111111111111111111111111111111111111111", Chain::Ethereum)
        .await
        .unwrap();
    assert_eq!(clean, ScreenResult::Clean);

    let bad = engine
        .screen_address("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef", Chain::Ethereum)
        .await
        .unwrap();
    assert!(matches!(bad, ScreenResult::Sanctioned { .. }));
}

#[tokio::test]
async fn velocity_limit_is_enforced() {
    let engine = MockPolicyEngine::new();
    let user = Uuid::new_v4();

    engine
        .set_limits(user, Limits {
            per_tx_usd: Decimal::new(10_000, 0),
            daily_withdrawal_usd: Decimal::new(1_000_000, 0),
            velocity_per_hour: 3,
        })
        .await
        .unwrap();

    for _ in 0..3 {
        engine
            .record(TxRecord {
                tx_id: Uuid::new_v4(),
                user_id: user,
                intent: make_intent(user, Decimal::new(10, 0)),
                usd_value: Decimal::new(10, 0),
                timestamp: Utc::now(),
            })
            .await
            .unwrap();
    }

    let intent = make_intent(user, Decimal::new(10, 0));
    let result = engine.evaluate(&intent).await;
    assert!(matches!(
        result,
        Err(policy_engine::PolicyError::VelocityExceeded { .. })
    ));
}

#[tokio::test]
async fn daily_total_aggregates_correctly() {
    let engine = MockPolicyEngine::new();
    let user = Uuid::new_v4();

    for value in [100, 200, 300] {
        engine
            .record(TxRecord {
                tx_id: Uuid::new_v4(),
                user_id: user,
                intent: make_intent(user, Decimal::from(value)),
                usd_value: Decimal::from(value),
                timestamp: Utc::now(),
            })
            .await
            .unwrap();
    }

    let total = engine.daily_total_usd(user).await.unwrap();
    assert_eq!(total, Decimal::from(600));
}

#[tokio::test]
async fn get_and_set_limits_roundtrip() {
    let engine = MockPolicyEngine::new();
    let user = Uuid::new_v4();

    let custom = Limits {
        per_tx_usd: Decimal::new(500, 0),
        daily_withdrawal_usd: Decimal::new(2_000, 0),
        velocity_per_hour: 5,
    };

    engine.set_limits(user, custom.clone()).await.unwrap();
    let fetched = engine.get_limits(user).await.unwrap();

    assert_eq!(fetched.per_tx_usd, custom.per_tx_usd);
    assert_eq!(fetched.daily_withdrawal_usd, custom.daily_withdrawal_usd);
    assert_eq!(fetched.velocity_per_hour, custom.velocity_per_hour);
}
