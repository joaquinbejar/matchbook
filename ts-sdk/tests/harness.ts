// A local validator (LiteSVM) running the compiled program, plus helpers to
// create mints, fund wallets and send transactions with kit.
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import {
  appendTransactionMessageInstructions,
  createTransactionMessage,
  generateKeyPairSigner,
  lamports,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  type Address,
  type Instruction,
  type KeyPairSigner,
} from '@solana/kit';
import { getCreateAccountInstruction } from '@solana-program/system';
import {
  getCreateAssociatedTokenIdempotentInstructionAsync,
  getInitializeMint2Instruction,
  getMintSize,
  getMintToInstruction,
} from '@solana-program/token';
import { FailedTransactionMetadata, LiteSVM, type TransactionMetadata } from 'litesvm';

import {
  MATCHBOOK_PROGRAM_ADDRESS,
  TOKEN_PROGRAM_ADDRESS,
  decodeMarket,
  findAssociatedTokenAddress,
  findVault,
  findVaultAuthority,
  type MarketAccounts,
  type MarketView,
} from '../src/index.js';

export const SO_PATH = fileURLToPath(new URL('../../target/deploy/matchbook_program.so', import.meta.url));
export const haveProgram = existsSync(SO_PATH);

export class Harness {
  readonly svm = new LiteSVM();
  payer!: KeyPairSigner;

  static async create(): Promise<Harness> {
    const h = new Harness();
    h.svm.addProgramFromFile(MATCHBOOK_PROGRAM_ADDRESS, SO_PATH);
    h.payer = await h.wallet();
    return h;
  }

  async wallet(): Promise<KeyPairSigner> {
    const k = await generateKeyPairSigner();
    this.svm.airdrop(k.address, lamports(10_000_000_000n));
    return k;
  }

  /** Sends `ixs`, signed by the payer and any signers in the instructions. */
  async send(ixs: Instruction[]): Promise<TransactionMetadata> {
    this.svm.expireBlockhash();
    const msg = pipe(
      createTransactionMessage({ version: 0 }),
      (m) => setTransactionMessageFeePayerSigner(this.payer, m),
      (m) =>
        setTransactionMessageLifetimeUsingBlockhash(
          { blockhash: this.svm.latestBlockhash(), lastValidBlockHeight: 0n },
          m,
        ),
      (m) => appendTransactionMessageInstructions(ixs, m),
    );
    const tx = await signTransactionMessageWithSigners(msg);
    const r = this.svm.sendTransaction(tx);
    if (r instanceof FailedTransactionMetadata) {
      throw new TxError(r);
    }
    return r;
  }

  async mint(decimals: number): Promise<Address> {
    const mint = await generateKeyPairSigner();
    const space = BigInt(getMintSize());
    await this.send([
      getCreateAccountInstruction({
        payer: this.payer,
        newAccount: mint,
        lamports: this.svm.minimumBalanceForRentExemption(space),
        space,
        programAddress: TOKEN_PROGRAM_ADDRESS,
      }),
      getInitializeMint2Instruction({ mint: mint.address, decimals, mintAuthority: this.payer.address }),
    ]);
    return mint.address;
  }

  /** Creates `owner`'s token account for `mint` and mints `amount` into it. */
  async fund(owner: Address, mint: Address, amount: bigint): Promise<Address> {
    const ata = await findAssociatedTokenAddress(owner, mint);
    const ixs: Instruction[] = [
      await getCreateAssociatedTokenIdempotentInstructionAsync({ payer: this.payer, owner, mint }),
    ];
    if (amount > 0n) {
      ixs.push(getMintToInstruction({ mint, token: ata, mintAuthority: this.payer, amount }));
    }
    await this.send(ixs);
    return ata;
  }

  tokenBalance(account: Address): bigint {
    const a = this.svm.getAccount(account);
    if (!a.exists) return 0n;
    return new DataView(a.data.buffer, a.data.byteOffset).getBigUint64(64, true);
  }

  market(address: Address): MarketView {
    const a = this.svm.getAccount(address);
    if (!a.exists) throw new Error('no market');
    return decodeMarket(address, new Uint8Array(a.data));
  }

  async marketAccounts(market: Address, baseMint: Address, quoteMint: Address): Promise<MarketAccounts> {
    return {
      market,
      baseMint,
      quoteMint,
      baseVault: await findVault(market, baseMint),
      quoteVault: await findVault(market, quoteMint),
      vaultAuthority: await findVaultAuthority(market),
      baseTokenProgram: TOKEN_PROGRAM_ADDRESS,
      quoteTokenProgram: TOKEN_PROGRAM_ADDRESS,
    };
  }
}

/** A failed transaction, with the custom error code when there is one. */
export class TxError extends Error {
  readonly code?: number;
  readonly logs: string[];

  constructor(r: FailedTransactionMetadata) {
    super(r.toString());
    // An instruction error wraps the instruction's own error, which for a
    // custom error carries its code.
    const e = r.err() as { err?: () => unknown };
    const inner = typeof e.err === 'function' ? (e.err() as { code?: unknown }) : undefined;
    if (inner && typeof inner.code === 'number') this.code = inner.code;
    this.logs = r.meta().logs();
  }
}

/** The custom error code a promise rejects with. */
export async function errorCode(p: Promise<unknown>): Promise<number | undefined> {
  try {
    await p;
  } catch (e) {
    if (e instanceof TxError) return e.code;
    throw e;
  }
  throw new Error('expected the transaction to fail');
}

/** Reads a u64 return value. */
export function returnedU64(r: TransactionMetadata, index = 0): bigint {
  const d = r.returnData().data();
  return new DataView(d.buffer, d.byteOffset).getBigUint64(index * 8, true);
}
