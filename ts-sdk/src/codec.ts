import { getAddressDecoder, getAddressEncoder, type Address } from '@solana/kit';

const addressEncoder = getAddressEncoder();
const addressDecoder = getAddressDecoder();

/** Little-endian writer into a fixed buffer. */
export class Writer {
  readonly bytes: Uint8Array;
  private readonly view: DataView;
  private at = 0;

  constructor(size: number) {
    this.bytes = new Uint8Array(size);
    this.view = new DataView(this.bytes.buffer);
  }

  u8(v: number): this {
    this.view.setUint8(this.at, v);
    this.at += 1;
    return this;
  }

  u16(v: number): this {
    this.view.setUint16(this.at, v, true);
    this.at += 2;
    return this;
  }

  u32(v: number): this {
    this.view.setUint32(this.at, v >>> 0, true);
    this.at += 4;
    return this;
  }

  u64(v: bigint): this {
    this.view.setBigUint64(this.at, BigInt.asUintN(64, v), true);
    this.at += 8;
    return this;
  }

  address(a: Address): this {
    this.bytes.set(addressEncoder.encode(a), this.at);
    this.at += 32;
    return this;
  }

  raw(b: Uint8Array): this {
    this.bytes.set(b, this.at);
    this.at += b.length;
    return this;
  }
}

/** Little-endian reads at fixed offsets. */
export class Reader {
  private readonly view: DataView;

  constructor(readonly bytes: Uint8Array) {
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  }

  u8(at: number): number {
    return this.view.getUint8(at);
  }

  u16(at: number): number {
    return this.view.getUint16(at, true);
  }

  u32(at: number): number {
    return this.view.getUint32(at, true);
  }

  u64(at: number): bigint {
    return this.view.getBigUint64(at, true);
  }

  address(at: number): Address {
    return addressDecoder.decode(this.bytes.subarray(at, at + 32));
  }

  bytes32(at: number): Uint8Array {
    return this.bytes.subarray(at, at + 32);
  }
}

/** Encodes an address to its 32 bytes. */
export function addressBytes(a: Address): Uint8Array {
  return new Uint8Array(addressEncoder.encode(a));
}
