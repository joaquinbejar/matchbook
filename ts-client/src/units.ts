/**
 * Conversions between human amounts and a market's lots and atoms.
 *
 * Prices on the book are quote lots per base lot. Commands take and print
 * quote tokens per base token, converted exactly with bigint math.
 */

/** A non-negative decimal held exactly: `mantissa / 10^scale`. */
export interface Decimal {
  mantissa: bigint;
  scale: number;
}

/** Parses `"150"`, `"150.25"` or `".5"`. */
export function parseDecimal(input: string): Decimal {
  const s = input.trim().replaceAll('_', '');
  const [int = '', frac = '', extra] = s.split('.');
  if (extra !== undefined || (int === '' && frac === '') || !/^\d*$/.test(int) || !/^\d*$/.test(frac)) {
    throw new Error(`not a decimal number: ${input}`);
  }
  return { mantissa: BigInt(`${int}${frac}` || '0'), scale: frac.length };
}

const pow10 = (e: number): bigint => 10n ** BigInt(e);

/** `d * num / den`, rounded down. */
function scaled(d: Decimal, num: bigint, den: bigint): bigint {
  return (d.mantissa * num) / (den * pow10(d.scale));
}

/** `d * num / den`, rounded to nearest. */
function scaledRound(d: Decimal, num: bigint, den: bigint): bigint {
  const q = den * pow10(d.scale);
  return (d.mantissa * num + q / 2n) / q;
}

/** Formats `v / 10^decimals` without trailing zeros. */
export function formatAtoms(v: bigint, decimals: number): string {
  const p = pow10(decimals);
  const int = v / p;
  const frac = v % p;
  if (frac === 0n) return int.toString();
  return `${int}.${frac.toString().padStart(decimals, '0').replace(/0+$/, '')}`;
}

/** Lot sizes and decimals of a market. */
export class Units {
  constructor(
    readonly baseLotSize: bigint,
    readonly quoteLotSize: bigint,
    readonly baseDecimals: number,
    readonly quoteDecimals: number,
  ) {}

  private get baseOne(): bigint {
    return pow10(this.baseDecimals);
  }

  private get quoteOne(): bigint {
    return pow10(this.quoteDecimals);
  }

  /** Book price of `price` quote tokens per base token, nearest tick. */
  priceToLots(price: string): bigint {
    const lots = scaledRound(parseDecimal(price), this.quoteOne * this.baseLotSize, this.baseOne * this.quoteLotSize);
    if (lots === 0n) throw new Error(`price ${price} is below one tick (${this.priceToString(1n)})`);
    return lots;
  }

  /** Book price of a float price, nearest tick (for the bot). */
  priceNumberToLots(price: number): bigint {
    const v =
      (price * Number(this.quoteOne) * Number(this.baseLotSize)) / (Number(this.baseOne) * Number(this.quoteLotSize));
    return Number.isFinite(v) && v > 0 ? BigInt(Math.round(v)) : 0n;
  }

  /** Quote tokens per base token of a book price. */
  priceToNumber(lots: bigint): number {
    return (
      (Number(lots) * Number(this.quoteLotSize) * Number(this.baseOne)) /
      (Number(this.baseLotSize) * Number(this.quoteOne))
    );
  }

  /** Prints a book price in quote tokens per base token. */
  priceToString(lots: bigint): string {
    const extra = 6;
    const v = (lots * this.quoteLotSize * this.baseOne * pow10(extra)) / this.baseLotSize;
    return formatAtoms(v, this.quoteDecimals + extra);
  }

  /** Base lots in `size` base tokens, rounded down. */
  sizeToLots(size: string): bigint {
    const lots = scaled(parseDecimal(size), this.baseOne, this.baseLotSize);
    if (lots === 0n) throw new Error(`size ${size} is below one lot (${this.lotsToString(1n)})`);
    return lots;
  }

  /** Prints base lots as base tokens. */
  lotsToString(lots: bigint): string {
    return formatAtoms(lots * this.baseLotSize, this.baseDecimals);
  }

  /** Base tokens in base lots. */
  lotsToNumber(lots: bigint): number {
    return (Number(lots) * Number(this.baseLotSize)) / Number(this.baseOne);
  }

  /** Base lots in a float number of base tokens, rounded down. */
  numberToLots(tokens: number): bigint {
    if (!(Number.isFinite(tokens) && tokens > 0)) return 0n;
    return BigInt(Math.floor((tokens * Number(this.baseOne)) / Number(this.baseLotSize) + 1e-9));
  }

  /** Prints base atoms as tokens. */
  baseToString(atoms: bigint): string {
    return formatAtoms(atoms, this.baseDecimals);
  }

  /** Prints quote atoms as tokens. */
  quoteToString(atoms: bigint): string {
    return formatAtoms(atoms, this.quoteDecimals);
  }

  /** Base atoms as tokens. */
  baseToNumber(atoms: bigint): number {
    return Number(atoms) / Number(this.baseOne);
  }

  /** Quote atoms as tokens. */
  quoteToNumber(atoms: bigint): number {
    return Number(atoms) / Number(this.quoteOne);
  }
}

/** Atoms of `amount` tokens with `decimals`. */
export function amountToAtoms(amount: string, decimals: number): bigint {
  return scaled(parseDecimal(amount), pow10(decimals), 1n);
}

/** Parses 90s, 30m, 12h, 7d or plain seconds. */
export function parseDuration(s: string): bigint {
  const m = /^(\d+)([smhd]?)$/.exec(s.trim());
  if (!m) throw new Error(`bad duration ${s}`);
  const mult = { '': 1n, s: 1n, m: 60n, h: 3_600n, d: 86_400n }[m[2] as '' | 's' | 'm' | 'h' | 'd'];
  return BigInt(m[1]!) * mult;
}
