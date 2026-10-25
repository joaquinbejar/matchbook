// Creates a demo SOL/USDC-style market on a cluster where MatchBook is
// deployed, quotes it with a market maker's reverse-order grid, and runs one
// swap from a plain wallet.
//
//   RPC_URL=https://api.devnet.solana.com npx tsx examples/demo-market.ts
//
// The payer is the Solana CLI wallet (~/.config/solana/id.json, or KEYPAIR).
// Test mints are created with the payer as mint authority.
import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';

import {
  appendTransactionMessageInstructions,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  generateKeyPairSigner,
  getSignatureFromTransaction,
  lamports,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type Address,
  type Instruction,
  type KeyPairSigner,
  type Signature,
} from '@solana/kit';
import { getCreateAccountInstruction } from '@solana-program/system';
import {
  getCreateAssociatedTokenIdempotentInstructionAsync,
  getInitializeMint2Instruction,
  getMintSize,
  getMintToInstruction,
} from '@solana-program/token';

import {
  OrderType,
  Side,
  TOKEN_PROGRAM_ADDRESS,
  batchUpdate,
  claimSeat,
  createMarket,
  decodeMarket,
  deposit,
  findAssociatedTokenAddress,
  findVault,
  findVaultAuthority,
  parseEventsFromLogs,
  swap,
  type MarketAccounts,
} from '../src/index.js';

const RPC_URL = process.env.RPC_URL ?? 'http://127.0.0.1:8899';
const WS_URL =
  process.env.WS_URL ?? RPC_URL.replace(/^http/, 'ws').replace(/:8899\b/, ':8900');
const KEYPAIR = process.env.KEYPAIR ?? `${homedir()}/.config/solana/id.json`;

const rpc = createSolanaRpc(RPC_URL);
const rpcSubscriptions = createSolanaRpcSubscriptions(WS_URL);
const sendAndConfirm = sendAndConfirmTransactionFactory({ rpc, rpcSubscriptions });

// 1 base lot = 0.001 base token (9 decimals); 1 quote lot = 0.000001 quote
// token (6 decimals). A price of 150,000 quote lots per base lot is 150.000.
const BASE_LOT = 1_000_000n;
const PRICE = (p: number) => BigInt(Math.round(p * 1000));

async function send(payer: KeyPairSigner, ixs: Instruction[]): Promise<Signature> {
  const { value: blockhash } = await rpc.getLatestBlockhash().send();
  const msg = pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(payer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(blockhash, m),
    (m) => appendTransactionMessageInstructions(ixs, m),
  );
  const tx = await signTransactionMessageWithSigners(msg);
  // The message has a blockhash lifetime, which the signed type does not carry.
  await sendAndConfirm(tx as Parameters<typeof sendAndConfirm>[0], { commitment: 'confirmed' });
  return getSignatureFromTransaction(tx);
}

async function mint(payer: KeyPairSigner, decimals: number): Promise<Address> {
  const m = await generateKeyPairSigner();
  const space = BigInt(getMintSize());
  const rent = await rpc.getMinimumBalanceForRentExemption(space).send();
  await send(payer, [
    getCreateAccountInstruction({
      payer,
      newAccount: m,
      lamports: rent,
      space,
      programAddress: TOKEN_PROGRAM_ADDRESS,
    }),
    getInitializeMint2Instruction({ mint: m.address, decimals, mintAuthority: payer.address }),
  ]);
  return m.address;
}

async function fund(payer: KeyPairSigner, owner: Address, m: Address, amount: bigint): Promise<Address> {
  const ata = await findAssociatedTokenAddress(owner, m);
  const ixs: Instruction[] = [
    await getCreateAssociatedTokenIdempotentInstructionAsync({ payer, owner, mint: m }),
  ];
  if (amount > 0n) ixs.push(getMintToInstruction({ mint: m, token: ata, mintAuthority: payer, amount }));
  await send(payer, ixs);
  return ata;
}

async function marketView(address: Address) {
  const { value } = await rpc.getAccountInfo(address, { encoding: 'base64' }).send();
  if (!value) throw new Error('market not found');
  return decodeMarket(address, new Uint8Array(Buffer.from(value.data[0], 'base64')));
}

async function main() {
  const payer = await createKeyPairSignerFromBytes(
    new Uint8Array(JSON.parse(readFileSync(KEYPAIR, 'utf8')) as number[]),
  );
  if (RPC_URL.includes('127.0.0.1') || RPC_URL.includes('localhost')) {
    await rpc.requestAirdrop(payer.address, lamports(10_000_000_000n)).send();
    await new Promise((r) => setTimeout(r, 1000));
  }
  console.log(`payer ${payer.address} on ${RPC_URL}`);

  const baseMint = await mint(payer, 9);
  const quoteMint = await mint(payer, 6);
  const marketKp = await generateKeyPairSigner();
  const market = marketKp.address;
  await send(payer, [
    await createMarket({
      payer,
      market: marketKp,
      baseMint,
      quoteMint,
      authority: payer.address,
      baseLotSize: BASE_LOT,
      quoteLotSize: 1n,
      takerFeeBps: 10,
      makerRebateBps: 2,
      referralShareBps: 2_000,
      initialNodes: 64,
    }),
  ]);
  const accounts: MarketAccounts = {
    market,
    baseMint,
    quoteMint,
    baseVault: await findVault(market, baseMint),
    quoteVault: await findVault(market, quoteMint),
    vaultAuthority: await findVaultAuthority(market),
    baseTokenProgram: TOKEN_PROGRAM_ADDRESS,
    quoteTokenProgram: TOKEN_PROGRAM_ADDRESS,
  };
  console.log(`market ${market}  base ${baseMint}  quote ${quoteMint}`);

  // The payer makes the market: a seat, deposits, and a grid of reverse
  // orders 0.2% wide, so every fill re-posts on the other side.
  const baseAta = await fund(payer, payer.address, baseMint, 10n * 10n ** 9n);
  const quoteAta = await fund(payer, payer.address, quoteMint, 5_000n * 10n ** 6n);
  await send(payer, [claimSeat({ payer, owner: payer, market })]);
  const seat = (await marketView(market)).findSeat(payer.address)!.index;
  const grid = [];
  for (let i = 0; i < 5; i++) {
    const common = { baseLots: 100n, orderType: OrderType.PostOnly, reverseSpread: 2_000 };
    grid.push({ side: Side.Ask, price: PRICE(150.1 + i * 0.1), ...common });
    grid.push({ side: Side.Bid, price: PRICE(149.9 - i * 0.1), ...common });
  }
  await send(payer, [
    deposit({ owner: payer, market, seat, userToken: baseAta, vault: accounts.baseVault, mint: baseMint, amount: 5n * 10n ** 9n }),
    deposit({ owner: payer, market, seat, userToken: quoteAta, vault: accounts.quoteVault, mint: quoteMint, amount: 2_000n * 10n ** 6n }),
    batchUpdate({ owner: payer, market, seat, orders: grid }),
  ]);

  // A wallet with no seat buys with 30 quote tokens, at an exact quote.
  const user = await generateKeyPairSigner();
  const userBase = await fund(payer, user.address, baseMint, 0n);
  const userQuote = await fund(payer, user.address, quoteMint, 30n * 10n ** 6n);
  const budget = 30n * 10n ** 6n;
  const quote = (await marketView(market)).quoteSwap({ side: Side.Bid, baseLots: 1_000_000n, quoteBudget: budget });
  const sig = await send(payer, [
    swap({
      user,
      accounts,
      userBase,
      userQuote,
      side: Side.Bid,
      baseLots: 1_000_000n,
      quoteBudget: budget,
      limitPrice: PRICE(1_000),
      minOut: quote.baseAtoms,
    }),
  ]);
  const tx = await rpc
    .getTransaction(sig, { commitment: 'confirmed', maxSupportedTransactionVersion: 0, encoding: 'json' })
    .send();
  const events = parseEventsFromLogs(tx?.meta?.logMessages ?? []);
  console.log(`swap ${sig}: quoted ${quote.baseAtoms} base atoms for ${quote.quoteNet} quote atoms`);
  for (const e of events) {
    if (e.kind === 'fill') console.log(`  fill ${e.baseLots} lots at ${e.price}`);
    if (e.kind === 'reversed') console.log(`  reversed into a ${Side[e.side]} of ${e.baseLots} lots at ${e.price}`);
  }

  const view = await marketView(market);
  console.log('asks', view.levels(Side.Ask, 5).map((l) => `${l.price}x${l.baseLots}`).join(' '));
  console.log('bids', view.levels(Side.Bid, 5).map((l) => `${l.price}x${l.baseLots}`).join(' '));
  if (RPC_URL.includes('devnet')) {
    console.log(`https://explorer.solana.com/address/${market}?cluster=devnet`);
  }
  process.exit(0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
