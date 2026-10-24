import { getProgramDerivedAddress, type Address } from '@solana/kit';

import { addressBytes } from './codec.js';
import {
  ASSOCIATED_TOKEN_PROGRAM_ADDRESS,
  MATCHBOOK_PROGRAM_ADDRESS,
  TOKEN_PROGRAM_ADDRESS,
  VAULT_AUTHORITY_SEED,
} from './constants.js';

/** The PDA that owns a market's vaults. */
export async function findVaultAuthority(
  market: Address,
  programAddress: Address = MATCHBOOK_PROGRAM_ADDRESS,
): Promise<Address> {
  const [pda] = await getProgramDerivedAddress({
    programAddress,
    seeds: [VAULT_AUTHORITY_SEED, addressBytes(market)],
  });
  return pda;
}

/** The associated token account of `owner` for `mint`. */
export async function findAssociatedTokenAddress(
  owner: Address,
  mint: Address,
  tokenProgram: Address = TOKEN_PROGRAM_ADDRESS,
): Promise<Address> {
  const [ata] = await getProgramDerivedAddress({
    programAddress: ASSOCIATED_TOKEN_PROGRAM_ADDRESS,
    seeds: [addressBytes(owner), addressBytes(tokenProgram), addressBytes(mint)],
  });
  return ata;
}

/** A market's vault for `mint`: the vault authority's associated token account. */
export async function findVault(
  market: Address,
  mint: Address,
  tokenProgram: Address = TOKEN_PROGRAM_ADDRESS,
  programAddress: Address = MATCHBOOK_PROGRAM_ADDRESS,
): Promise<Address> {
  return findAssociatedTokenAddress(
    await findVaultAuthority(market, programAddress),
    mint,
    tokenProgram,
  );
}
