//! The Jupiter adapter quotes exactly what the program executes.
#![cfg(feature = "jupiter")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    missing_docs
)]

use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, QuoteParams, SwapMode, SwapParams,
};
use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccountIdempotent, CreateMint, MintTo};
use matchbook_sdk::engine::{OrderType, Side};
use matchbook_sdk::ix::{self, CancelAll, MarketAccounts};
use matchbook_sdk::jupiter::MatchBookAmm;
use matchbook_sdk::{CreateMarketArgs, OrderArgs, NO_HINT, PROGRAM_ID, TOKEN_PROGRAM_ID};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

const LOT: u64 = 1_000_000;

struct Env {
    svm: LiteSVM,
    payer: Keypair,
    accts: MarketAccounts,
}

impl Env {
    fn new() -> Self {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(
            PROGRAM_ID,
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../target/deploy/matchbook_program.so"
            ),
        )
        .expect("run `cargo build-sbf --manifest-path program/Cargo.toml` first");
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();
        let base_mint = CreateMint::new(&mut svm, &payer)
            .decimals(9)
            .send()
            .unwrap();
        let quote_mint = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .send()
            .unwrap();
        let market = Keypair::new();
        let accts = MarketAccounts::new(
            market.pubkey(),
            base_mint,
            quote_mint,
            TOKEN_PROGRAM_ID,
            TOKEN_PROGRAM_ID,
        );
        let mut env = Self { svm, payer, accts };
        let args = CreateMarketArgs {
            base_lot_size: LOT,
            quote_lot_size: 1,
            min_base_lots: 1,
            taker_fee_bps: 10,
            maker_rebate_bps: 2,
            initial_nodes: 64,
            referral_share_bps: 0,
            flags: 0,
            authority: env.payer.pubkey().to_bytes(),
        };
        let create = ix::create_market(&env.payer.pubkey(), &env.accts, &args);
        env.send(&[create], &[&market]);
        env
    }

    fn send(&mut self, ixs: &[Instruction], signers: &[&Keypair]) {
        self.svm.expire_blockhash();
        let mut all: Vec<&Keypair> = vec![&self.payer];
        all.extend_from_slice(signers);
        let msg = Message::new(ixs, Some(&self.payer.pubkey()));
        let tx = Transaction::new(&all, msg, self.svm.latest_blockhash());
        self.svm
            .send_transaction(tx)
            .map_err(|e| format!("{:?}\n{}", e.err, e.meta.logs.join("\n")))
            .unwrap();
    }

    /// A funded wallet with token accounts for both mints.
    fn wallet(&mut self, base: u64, quote: u64) -> (Keypair, Pubkey, Pubkey) {
        let k = Keypair::new();
        self.svm.airdrop(&k.pubkey(), 1_000_000_000).unwrap();
        let mut ata = |mint: Pubkey, amount: u64| {
            let a = CreateAssociatedTokenAccountIdempotent::new(&mut self.svm, &self.payer, &mint)
                .owner(&k.pubkey())
                .send()
                .unwrap();
            if amount > 0 {
                MintTo::new(&mut self.svm, &self.payer, &mint, &a, amount)
                    .send()
                    .unwrap();
            }
            a
        };
        let b = ata(self.accts.base_mint, base);
        let q = ata(self.accts.quote_mint, quote);
        (k, b, q)
    }

    fn balance(&self, token: &Pubkey) -> u64 {
        let d = self.svm.get_account(token).unwrap().data;
        u64::from_le_bytes(d[64..72].try_into().unwrap())
    }

    fn now(&self) -> i64 {
        self.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp
    }

    fn amm(&self) -> MatchBookAmm {
        let account = self.svm.get_account(&self.accts.market).unwrap();
        let ctx = AmmContext::default();
        ctx.clock_ref
            .unix_timestamp
            .store(self.now(), std::sync::atomic::Ordering::Relaxed);
        MatchBookAmm::from_keyed_account(
            &KeyedAccount {
                key: self.accts.market,
                account: jup_account(account),
                params: None,
            },
            &ctx,
        )
        .unwrap()
    }
}

fn jup_account(a: solana_account::Account) -> jup_account::Account {
    jup_account::Account {
        lamports: a.lamports,
        data: a.data,
        owner: a.owner,
        executable: a.executable,
        rent_epoch: a.rent_epoch,
    }
}

fn post_only(side: Side, price: u64, lots: u64) -> OrderArgs {
    OrderArgs {
        order_type: OrderType::PostOnly as u8,
        ..OrderArgs::limit(side as u8, price, lots)
    }
}

/// A market maker with asks at 100.1, 100.2 (two orders) and 100.5, and
/// bids at 99.9 and 99.5, one of which expires soon.
fn seeded() -> Env {
    let mut env = Env::new();
    let (mm, mm_base, mm_quote) = env.wallet(100 * LOT, 100_000_000_000);
    let market = env.accts.market;
    let accts = env.accts;
    let expires = env.now() as u64 + 30;
    env.send(
        &[
            ix::claim_seat(&env.payer.pubkey(), &mm.pubkey(), &market, None),
            ix::deposit(
                &mm.pubkey(),
                &accts,
                NO_HINT,
                &accts.base_mint,
                &mm_base,
                100 * LOT,
            ),
            ix::deposit(
                &mm.pubkey(),
                &accts,
                NO_HINT,
                &accts.quote_mint,
                &mm_quote,
                100_000_000_000,
            ),
            ix::batch_update(
                &mm.pubkey(),
                &market,
                NO_HINT,
                CancelAll::None,
                &[],
                &[
                    post_only(Side::Ask, 100_100, 5),
                    post_only(Side::Ask, 100_200, 5),
                    post_only(Side::Ask, 100_200, 3),
                    post_only(Side::Ask, 100_500, 20),
                    OrderArgs {
                        expires_at: expires,
                        ..post_only(Side::Bid, 99_900, 4)
                    },
                    post_only(Side::Bid, 99_500, 10),
                ],
            ),
        ],
        &[&mm],
    );
    env
}

fn quote(
    amm: &MatchBookAmm,
    input: Pubkey,
    output: Pubkey,
    amount: u64,
) -> jupiter_amm_interface::Quote {
    amm.quote(&QuoteParams {
        amount,
        input_mint: input,
        output_mint: output,
        swap_mode: SwapMode::ExactIn,
        fee_mode: Default::default(),
    })
    .unwrap()
}

#[test]
fn buy_quote_matches_execution() {
    let mut env = seeded();
    let amm = env.amm();
    let a = env.accts;
    let budget = 1_500_000;
    let q = quote(&amm, a.quote_mint, a.base_mint, budget);
    // 5 lots at 100,100, 8 at 100,200 and 1 at 100,500 fit in the budget.
    assert_eq!(q.out_amount, 14 * LOT);
    assert!(q.in_amount <= budget);
    assert_eq!(q.fee_mint, a.quote_mint);

    let (user, ub, uq) = env.wallet(0, budget);
    let swap = amm.swap_instruction(&user.pubkey(), &ub, &uq, Side::Bid, budget, q.out_amount);
    env.send(&[swap], &[&user]);
    assert_eq!(env.balance(&ub), q.out_amount);
    assert_eq!(budget - env.balance(&uq), q.in_amount);
}

#[test]
fn sell_quote_matches_execution_and_skips_expired_orders() {
    let mut env = seeded();
    // Past the bid's expiry: the quote must skip it, as the program does.
    let mut clock = env.svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += 60;
    env.svm.set_sysvar(&clock);
    let amm = env.amm();
    let a = env.accts;
    let amount = 7 * LOT + 123; // the odd atoms are not sold
    let q = quote(&amm, a.base_mint, a.quote_mint, amount);
    assert_eq!(q.in_amount, 7 * LOT);
    let gross = 7 * 99_500;
    assert!((696..=697).contains(&q.fee_amount), "10 bps of {gross}");
    assert_eq!(q.out_amount, gross - q.fee_amount);

    let (user, ub, uq) = env.wallet(amount, 0);
    let swap = amm.swap_instruction(&user.pubkey(), &ub, &uq, Side::Ask, amount, q.out_amount);
    env.send(&[swap], &[&user]);
    assert_eq!(env.balance(&uq), q.out_amount);
    assert_eq!(amount - env.balance(&ub), q.in_amount);
}

#[test]
fn update_follows_the_book_and_metas_match_the_instruction() {
    let mut env = seeded();
    let mut amm = env.amm();
    let a = env.accts;
    let before = quote(&amm, a.quote_mint, a.base_mint, 600_000);

    let (user, ub, uq) = env.wallet(0, 600_000);
    env.send(
        &[amm.swap_instruction(&user.pubkey(), &ub, &uq, Side::Bid, 600_000, 0)],
        &[&user],
    );
    let mut map = AccountMap::default();
    for k in amm.get_accounts_to_update() {
        map.insert(k, jup_account(env.svm.get_account(&k).unwrap()));
    }
    amm.update(&map).unwrap();
    let after = quote(&amm, a.quote_mint, a.base_mint, 600_000);
    // The 100.1 asks are gone: the same 5 lots now cost 100.2 each.
    assert_eq!(after.out_amount, before.out_amount);
    assert!(after.in_amount > before.in_amount);

    let jup = Pubkey::new_unique();
    let metas = amm
        .get_swap_and_account_metas(&SwapParams {
            swap_mode: SwapMode::ExactIn,
            in_amount: 600_000,
            out_amount: after.out_amount,
            source_mint: a.quote_mint,
            destination_mint: a.base_mint,
            source_token_account: uq,
            destination_token_account: ub,
            token_transfer_authority: user.pubkey(),
            user: user.pubkey(),
            payer: user.pubkey(),
            quote_mint_to_referrer: None,
            jupiter_program_id: &jup,
            missing_dynamic_accounts_as_default: false,
        })
        .unwrap()
        .account_metas;
    let direct = amm.swap_instruction(&user.pubkey(), &ub, &uq, Side::Bid, 600_000, 0);
    assert_eq!(metas, direct.accounts);
    assert_eq!(metas.len(), amm.get_accounts_len());
    assert_eq!(amm.get_reserve_mints(), vec![a.base_mint, a.quote_mint]);
    assert!(amm.is_active());
}
