/** RPC access: sending transactions, reading markets, wallets and tokens. */
import { generateKeyPairSync } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';

import {
  appendTransactionMessageInstructions,
  blockhash,
  compileTransaction,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createTransactionMessage,
  generateKeyPairSigner,
  getBase64EncodedWireTransaction,
  getSignatureFromTransaction,
  isTransactionWithinSizeLimit,
  lamports,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  addSignersToTransactionMessage,
  type Address,
  type Instruction,
  type KeyPairSigner,
  type Rpc,
  type Signature,
  type SolanaRpcApi,
  type TransactionSigner,
} from '@solana/kit';
import { getSetComputeUnitLimitInstruction, getSetComputeUnitPriceInstruction } from '@solana-program/compute-budget';
import { getCreateAccountInstruction } from '@solana-program/system';
import {
  getCreateAssociatedTokenIdempotentInstruction,
  getInitializeMint2Instruction,
  getMintToInstruction,
} from '@solana-program/token';
import {
  ASSOCIATED_TOKEN_PROGRAM_ADDRESS,
  TOKEN_2022_PROGRAM_ADDRESS,
  TOKEN_PROGRAM_ADDRESS,
  decodeMarket,
  findAssociatedTokenAddress,
  findVaultAuthority,
  matchbookErrorName,
  parseEventsFromLogs,
  type MarketAccounts,
  type MarketView,
  type MatchbookEvent,
} from '@matchbook/sdk';

import { Units } from './units.js';

/** Resolves a cluster name to an RPC URL. Anything else is taken as a URL. */
export function clusterUrl(cluster: string): string {
  switch (cluster) {
    case 'localnet':
    case 'localhost':
    case 'l':
      return 'http://127.0.0.1:8899';
    case 'devnet':
    case 'd':
      return 'https://api.devnet.solana.com';
    case 'mainnet':
    case 'mainnet-beta':
    case 'm':
      return 'https://api.mainnet-beta.solana.com';
    case 'testnet':
    case 't':
      return 'https://api.testnet.solana.com';
    default:
      return cluster;
  }
}

/** The Solana CLI default wallet. */
export const DEFAULT_KEYPAIR = `${homedir()}/.config/solana/id.json`;

/** Reads a keypair file (a JSON array of 64 bytes). */
export async function loadKeypair(path: string): Promise<KeyPairSigner> {
  let bytes: number[];
  try {
    bytes = JSON.parse(readFileSync(path, 'utf8')) as number[];
  } catch (e) {
    throw new Error(`cannot read keypair ${path}: ${(e as Error).message}`, { cause: e });
  }
  return createKeyPairSignerFromBytes(new Uint8Array(bytes));
}

/** Creates a keypair and writes it in the Solana CLI format. */
export async function newKeypairFile(path: string): Promise<KeyPairSigner> {
  const { privateKey, publicKey } = generateKeyPairSync('ed25519');
  const d = Buffer.from(privateKey.export({ format: 'jwk' }).d!, 'base64url');
  const x = Buffer.from(publicKey.export({ format: 'jwk' }).x!, 'base64url');
  const bytes = new Uint8Array([...d, ...x]);
  writeFileSync(path, JSON.stringify([...bytes]), { mode: 0o600 });
  return createKeyPairSignerFromBytes(bytes);
}

/** Unix time in seconds. */
export const now = (): bigint => BigInt(Math.floor(Date.now() / 1000));

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** What a confirmed transaction did. */
export interface Executed {
  signature: Signature;
  /** The program's return data (a new order or stop id). */
  returnData?: Uint8Array;
  events: MatchbookEvent[];
  computeUnits?: bigint;
}

/** The u64 a transaction returned, such as a new order or stop id. */
export function returnedU64(e: Executed): bigint | undefined {
  if (!e.returnData || e.returnData.length < 8) return undefined;
  return new DataView(e.returnData.buffer, e.returnData.byteOffset, 8).getBigUint64(0, true);
}

/** A market with its accounts and units. */
export interface MarketCtx {
  view: MarketView;
  accounts: MarketAccounts;
  units: Units;
}

/** Seat index of `owner`, or an error telling how to claim one. */
export function requireSeat(m: MarketCtx, owner: Address): number {
  const s = m.view.findSeat(owner);
  if (!s) throw new Error(`${owner} has no seat on ${m.view.address}; run \`matchbook seat claim ${m.view.address}\``);
  return s.index;
}

/** Compute budget settings for every transaction. */
export interface Fees {
  cuLimit?: number;
  priorityMicroLamports?: bigint;
}

/** A connection to a cluster with a fee payer. */
export class Chain {
  readonly rpc: Rpc<SolanaRpcApi>;

  constructor(
    readonly url: string,
    readonly payer: KeyPairSigner,
    readonly fees: Fees = {},
  ) {
    this.rpc = createSolanaRpc(url);
  }

  private budget(): Instruction[] {
    const out: Instruction[] = [];
    if (this.fees.cuLimit) out.push(getSetComputeUnitLimitInstruction({ units: this.fees.cuLimit }));
    if (this.fees.priorityMicroLamports) {
      out.push(getSetComputeUnitPriceInstruction({ microLamports: this.fees.priorityMicroLamports }));
    }
    return out;
  }

  /**
   * Sends `ixs` paid by `feePayer` (default: the wallet) and waits for
   * confirmation. Signers named by the instructions sign automatically;
   * `extra` adds more.
   */
  /** Whether {@link Chain.send} could fit `ixs`, compute budget included, in one transaction. */
  fits(ixs: Instruction[], feePayer: TransactionSigner = this.payer, extra: TransactionSigner[] = []): boolean {
    const msg = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(feePayer, m),
      // Any blockhash has the same size.
      (m) => setTransactionMessageLifetimeUsingBlockhash({ blockhash: blockhash('11111111111111111111111111111111'), lastValidBlockHeight: 0n }, m),
      (m) => appendTransactionMessageInstructions([...this.budget(), ...ixs], m),
      (m) => (extra.length ? addSignersToTransactionMessage(extra, m) : m),
    );
    return isTransactionWithinSizeLimit(compileTransaction(msg));
  }

  async send(ixs: Instruction[], feePayer: TransactionSigner = this.payer, extra: TransactionSigner[] = []): Promise<Signature> {
    const { value: blockhash } = await this.rpc.getLatestBlockhash({ commitment: 'confirmed' }).send();
    const msg = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(feePayer, m),
      (m) => setTransactionMessageLifetimeUsingBlockhash(blockhash, m),
      (m) => appendTransactionMessageInstructions([...this.budget(), ...ixs], m),
      (m) => (extra.length ? addSignersToTransactionMessage(extra, m) : m),
    );
    const tx = await signTransactionMessageWithSigners(msg);
    const sig = getSignatureFromTransaction(tx);
    try {
      await this.rpc
        .sendTransaction(getBase64EncodedWireTransaction(tx), { encoding: 'base64', preflightCommitment: 'confirmed' })
        .send();
    } catch (e) {
      throw explain(e);
    }
    for (let i = 0; i < 120; i++) {
      const { value } = await this.rpc.getSignatureStatuses([sig]).send();
      const st = value[0];
      if (st?.err) throw new Error(`transaction ${sig} failed: ${JSON.stringify(st.err, bigintJson)}`);
      if (st && (st.confirmationStatus === 'confirmed' || st.confirmationStatus === 'finalized')) return sig;
      await sleep(250);
    }
    throw new Error(`transaction ${sig} was not confirmed`);
  }

  /** {@link Chain.send}, then reads return data and events. */
  async execute(ixs: Instruction[], feePayer?: TransactionSigner, extra?: TransactionSigner[]): Promise<Executed> {
    const signature = await this.send(ixs, feePayer, extra);
    for (let i = 0; i < 20; i++) {
      const tx = await this.rpc
        .getTransaction(signature, { commitment: 'confirmed', maxSupportedTransactionVersion: 0, encoding: 'json' })
        .send();
      if (tx) {
        const rd = tx.meta?.returnData?.data?.[0];
        return {
          signature,
          returnData: rd ? new Uint8Array(Buffer.from(rd, 'base64')) : undefined,
          events: parseEventsFromLogs(tx.meta?.logMessages ?? []),
          computeUnits: tx.meta?.computeUnitsConsumed === undefined ? undefined : BigInt(tx.meta.computeUnitsConsumed),
        };
      }
      await sleep(250);
    }
    return { signature, events: [] };
  }

  /** Raw account data, or undefined. */
  async accountData(address: Address): Promise<{ data: Uint8Array; owner: Address } | undefined> {
    const { value } = await this.rpc.getAccountInfo(address, { encoding: 'base64', commitment: 'confirmed' }).send();
    if (!value) return undefined;
    return { data: new Uint8Array(Buffer.from(value.data[0], 'base64')), owner: value.owner };
  }

  /** Reads and decodes a market, its accounts and units. */
  async market(address: Address): Promise<MarketCtx> {
    const acc = await this.accountData(address);
    if (!acc) throw new Error(`market ${address} not found`);
    const view = decodeMarket(address, acc.data);
    const h = view.header;
    const [base, quote] = await Promise.all([this.accountData(h.baseMint), this.accountData(h.quoteMint)]);
    if (!base || !quote) throw new Error('market mints not found');
    const accounts: MarketAccounts = {
      market: address,
      baseMint: h.baseMint,
      quoteMint: h.quoteMint,
      baseVault: h.baseVault,
      quoteVault: h.quoteVault,
      vaultAuthority: await findVaultAuthority(address),
      baseTokenProgram: base.owner,
      quoteTokenProgram: quote.owner,
    };
    const units = new Units(h.baseLotSize, h.quoteLotSize, base.data[44]!, quote.data[44]!);
    return { view, accounts, units };
  }

  /** Re-reads the market account of `m` in place. */
  async refresh(m: MarketCtx): Promise<void> {
    const acc = await this.accountData(m.view.address);
    if (!acc) throw new Error('market disappeared');
    m.view = decodeMarket(m.view.address, acc.data);
  }

  /** The token program that owns `mint`. */
  async tokenProgramOf(mint: Address): Promise<Address> {
    const a = await this.accountData(mint);
    if (!a) throw new Error(`mint ${mint} not found`);
    if (a.owner !== TOKEN_PROGRAM_ADDRESS && a.owner !== TOKEN_2022_PROGRAM_ADDRESS) {
      throw new Error(`${mint} is not a token mint`);
    }
    return a.owner;
  }

  /** Decimals of a mint. */
  async decimals(mint: Address): Promise<number> {
    const a = await this.accountData(mint);
    if (!a) throw new Error(`mint ${mint} not found`);
    return a.data[44]!;
  }

  /** Token balance in atoms, 0 if the account does not exist. */
  async tokenBalance(account: Address): Promise<bigint> {
    const a = await this.accountData(account);
    if (!a || a.data.length < 72) return 0n;
    return new DataView(a.data.buffer, a.data.byteOffset + 64, 8).getBigUint64(0, true);
  }

  /** Lamports of an account. */
  async solBalance(address: Address): Promise<bigint> {
    return (await this.rpc.getBalance(address, { commitment: 'confirmed' }).send()).value;
  }

  /** Requests an airdrop and waits for it. */
  async airdrop(to: Address, amount: bigint): Promise<Signature> {
    const sig = await this.rpc.requestAirdrop(to, lamports(amount)).send();
    for (let i = 0; i < 120; i++) {
      const { value } = await this.rpc.getSignatureStatuses([sig]).send();
      if (value[0]?.confirmationStatus === 'confirmed' || value[0]?.confirmationStatus === 'finalized') return sig;
      await sleep(250);
    }
    throw new Error(`airdrop ${sig} was not confirmed`);
  }

  /** Associated token account of `owner` for `mint`. */
  async ata(owner: Address, mint: Address, tokenProgram: Address): Promise<Address> {
    return findAssociatedTokenAddress(owner, mint, tokenProgram);
  }

  /** Instruction creating `owner`'s associated token account if missing. */
  async ensureAtaIx(owner: Address, mint: Address, tokenProgram: Address): Promise<Instruction> {
    return getCreateAssociatedTokenIdempotentInstruction(
      {
        payer: this.payer,
        ata: await this.ata(owner, mint, tokenProgram),
        owner,
        mint,
        tokenProgram,
      },
      { programAddress: ASSOCIATED_TOKEN_PROGRAM_ADDRESS },
    );
  }

  /** Creates a mint with the wallet as mint authority. */
  async createMint(decimals: number, tokenProgram: Address = TOKEN_PROGRAM_ADDRESS): Promise<Address> {
    const mint = await generateKeyPairSigner();
    // A mint without extensions is 82 bytes under both token programs.
    const space = 82n;
    const rent = await this.rpc.getMinimumBalanceForRentExemption(space).send();
    await this.send([
      getCreateAccountInstruction({ payer: this.payer, newAccount: mint, lamports: rent, space, programAddress: tokenProgram }),
      getInitializeMint2Instruction({ mint: mint.address, decimals, mintAuthority: this.payer.address }, { programAddress: tokenProgram }),
    ]);
    return mint.address;
  }

  /** Mints `atoms` to `owner`'s associated token account, creating it. */
  async mintTo(mint: Address, owner: Address, atoms: bigint): Promise<Address> {
    const tp = await this.tokenProgramOf(mint);
    const ata = await this.ata(owner, mint, tp);
    await this.send([
      await this.ensureAtaIx(owner, mint, tp),
      getMintToInstruction({ mint, token: ata, mintAuthority: this.payer, amount: atoms }, { programAddress: tp }),
    ]);
    return ata;
  }
}

/** JSON.stringify replacer that prints bigints. */
export const bigintJson = (_k: string, v: unknown) => (typeof v === 'bigint' ? v.toString() : v);

/** Adds the MatchBook error name to a failed send. */
function explain(e: unknown): Error {
  const err = e as Error & { context?: { logs?: string[] }; cause?: unknown };
  const text = JSON.stringify(err, bigintJson) + String(err.message) + JSON.stringify(err.cause ?? {}, bigintJson);
  const m = /"Custom":\s*(\d+)/.exec(text) ?? /custom program error: 0x([0-9a-f]+)/i.exec(text);
  if (m) {
    const code = m[0].includes('0x') ? parseInt(m[1]!, 16) : Number(m[1]);
    const name = matchbookErrorName(code) ?? 'unknown';
    return new Error(`MatchBook error ${code}: ${name} (${err.message})`);
  }
  return err instanceof Error ? err : new Error(String(e));
}
