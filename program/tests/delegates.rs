//! Session keys and other delegates acting for a seat.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

mod common;

use common::*;
use matchbook_engine::{order_id, permission};
use matchbook_program::instruction::{build, DelegateArgs};
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_signer::Signer;

const LOT: u64 = 1_000_000;
const TRADER_NOT_FOUND: u32 = 4;
const TOO_MANY_DELEGATES: u32 = 20;
const NOT_PERMITTED: u32 = 21;
const DELEGATE_EXPIRED: u32 = 22;
const INVALID_DELEGATE: u32 = 23;

/// A key with lamports for fees, acting as a delegate. Its token accounts are
/// never used, so they are left default.
fn key(h: &mut H) -> User {
    let kp = Keypair::new();
    h.svm.airdrop(&kp.pubkey(), 10_000_000_000).unwrap();
    User {
        kp,
        base: Default::default(),
        quote: Default::default(),
    }
}

fn set_delegate(
    h: &mut H,
    owner: &User,
    d: &User,
    expires_at: u64,
    permissions: u64,
) -> Option<u32> {
    let ix = h.ix(
        build::set_delegate(
            u32::MAX,
            &DelegateArgs {
                delegate: d.key().to_bytes(),
                expires_at,
                permissions,
            },
        ),
        h.owner_metas(owner),
    );
    err_code(&h.send(&[ix], &[&owner.kp]))
}

fn remove_delegate(h: &mut H, signer: &User, d: &User) -> Option<u32> {
    let ix = h.ix(
        build::remove_delegate(u32::MAX, &d.key().to_bytes()),
        vec![
            AccountMeta::new_readonly(signer.key(), true),
            AccountMeta::new(h.market, false),
        ],
    );
    err_code(&h.send(&[ix], &[&signer.kp]))
}

fn now(h: &H) -> u64 {
    let c: solana_clock::Clock = h.svm.get_sysvar();
    c.unix_timestamp as u64
}

fn newest_ask(h: &H) -> u64 {
    h.with_market(|m| {
        let i = m.best_ask();
        order_id(m.order(i).unwrap().seq, i)
    })
}

#[test]
fn session_key_trades_for_the_seat_but_cannot_withdraw() {
    let mut h = H::new(default_market(), false);
    let a = h.user(10 * LOT, 0);
    h.claim_seat(&a).unwrap();
    h.deposit(&a, true, 10 * LOT).unwrap();
    let session = key(&mut h);
    let t = now(&h);
    assert_eq!(
        set_delegate(&mut h, &a, &session, t + 3_600, permission::TRADE),
        None
    );

    // The session key places an order on a's seat, paid from a's balance.
    h.place(&session, limit(1, 100_000, 4)).unwrap();
    assert_eq!(h.trader(&a).base_free, 6 * LOT);
    let id = newest_ask(&h);
    let ix = h.cancel_ix(&session, build::cancel_order(u32::MAX, id));
    h.send(&[ix], &[&session.kp]).unwrap();
    assert_eq!(h.trader(&a).base_free, 10 * LOT);

    // It cannot withdraw, grant delegates or have a seat of its own here.
    assert!(h.withdraw(&session, true, 1).is_err());
    let other = key(&mut h);
    assert_eq!(
        set_delegate(&mut h, &session, &other, 0, permission::TRADE),
        Some(NOT_PERMITTED)
    );
    assert!(h.seat(&session).is_none());

    // After the session ends it can no longer act.
    let mut clock: solana_clock::Clock = h.svm.get_sysvar();
    clock.unix_timestamp += 7_200;
    h.svm.set_sysvar(&clock);
    assert_eq!(
        err_code(&h.place(&session, limit(1, 100_000, 1))),
        Some(DELEGATE_EXPIRED)
    );
    // Renewing the grant reopens it.
    let t = now(&h);
    assert_eq!(
        set_delegate(&mut h, &a, &session, t + 60, permission::TRADE),
        None
    );
    h.place(&session, limit(1, 100_000, 1)).unwrap();
    h.audit();
}

#[test]
fn cancel_only_delegate_and_revocation() {
    let mut h = H::new(default_market(), false);
    let a = h.user(10 * LOT, 0);
    h.claim_seat(&a).unwrap();
    h.deposit(&a, true, 10 * LOT).unwrap();
    h.place(&a, limit(1, 100_000, 4)).unwrap();
    let id = newest_ask(&h);

    let risk = key(&mut h);
    assert_eq!(set_delegate(&mut h, &a, &risk, 0, permission::CANCEL), None);
    assert_eq!(
        err_code(&h.place(&risk, limit(1, 100_000, 1))),
        Some(NOT_PERMITTED)
    );
    assert_eq!(
        err_code(&h.batch(&risk, 0, &[], &[limit(1, 100_000, 1)])),
        Some(NOT_PERMITTED)
    );
    h.batch(&risk, 0, &[id], &[]).unwrap();
    assert_eq!(h.trader(&a).base_free, 10 * LOT);

    // A delegate may revoke itself but no other delegate.
    let bot = key(&mut h);
    assert_eq!(set_delegate(&mut h, &a, &bot, 0, permission::ALL), None);
    assert_eq!(remove_delegate(&mut h, &risk, &bot), Some(NOT_PERMITTED));
    assert_eq!(remove_delegate(&mut h, &risk, &risk), None);
    assert_eq!(
        err_code(&h.batch(&risk, 0, &[], &[])),
        Some(TRADER_NOT_FOUND)
    );
    // The owner revokes the other.
    assert_eq!(remove_delegate(&mut h, &a, &bot), None);
    assert_eq!(
        err_code(&h.place(&bot, limit(1, 100_000, 1))),
        Some(TRADER_NOT_FOUND)
    );
    h.audit();
}

#[test]
fn delegate_limits() {
    let mut h = H::new(default_market(), false);
    let a = h.user(0, 0);
    let b = h.user(0, 0);
    h.claim_seat(&a).unwrap();
    h.claim_seat(&b).unwrap();
    // Not the owner itself, not unknown permissions.
    assert_eq!(
        set_delegate(&mut h, &a, &a, 0, permission::TRADE),
        Some(INVALID_DELEGATE)
    );
    let d = key(&mut h);
    assert_eq!(set_delegate(&mut h, &a, &d, 0, 0), Some(INVALID_DELEGATE));
    assert_eq!(
        set_delegate(&mut h, &a, &d, 0, 1 << 5),
        Some(INVALID_DELEGATE)
    );
    // A key delegates for one seat only, and cannot be another seat's owner.
    assert_eq!(set_delegate(&mut h, &a, &d, 0, permission::TRADE), None);
    assert!(set_delegate(&mut h, &b, &d, 0, permission::TRADE).is_some());
    assert!(set_delegate(&mut h, &a, &b, 0, permission::TRADE).is_some());
    for _ in 1..8 {
        let k = key(&mut h);
        assert_eq!(set_delegate(&mut h, &a, &k, 0, permission::TRADE), None);
    }
    let k = key(&mut h);
    assert_eq!(
        set_delegate(&mut h, &a, &k, 0, permission::TRADE),
        Some(TOO_MANY_DELEGATES)
    );
    h.audit();
}
