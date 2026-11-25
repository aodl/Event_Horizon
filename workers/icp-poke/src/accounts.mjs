import { createHash } from 'node:crypto';
import { Principal } from '@icp-sdk/core/principal';

const U64_MAX = (1n << 64n) - 1n;

function principalBytes(value) {
  return (typeof value === 'string' ? Principal.fromText(value) : value).toUint8Array();
}

function u64Bytes(value) {
  if (typeof value !== 'bigint' || value < 0n || value > U64_MAX) {
    throw new Error('Account target must be an unsigned 64-bit bigint');
  }
  const bytes = Buffer.alloc(8);
  bytes.writeBigUInt64BE(value);
  return bytes;
}

export function numberedSubaccount(number) {
  return Uint8Array.from(Buffer.concat([Buffer.alloc(24), u64Bytes(number)]));
}

export function neuronSubaccount(subject, nonce) {
  return Uint8Array.from(createHash('sha256')
    .update(Buffer.from([0x0c]))
    .update('neuron-stake', 'ascii')
    .update(principalBytes(subject))
    .update(u64Bytes(nonce))
    .digest());
}

// ICP AccountIdentifier: big-endian CRC32(SHA224(domain || owner || subaccount))
// followed by the SHA224 digest. This is not the ICRC-1 textual account format.
export function accountIdentifier(owner, subaccount = new Uint8Array(32)) {
  if (!(subaccount instanceof Uint8Array) || subaccount.length !== 32) {
    throw new Error('Subaccount must contain exactly 32 bytes');
  }
  const hash = createHash('sha224')
    .update(Buffer.from('\x0aaccount-id', 'ascii'))
    .update(principalBytes(owner))
    .update(subaccount)
    .digest();
  let crc = 0xffffffff;
  for (const byte of hash) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
    }
  }
  const checksum = Buffer.alloc(4);
  checksum.writeUInt32BE((crc ^ 0xffffffff) >>> 0);
  return Buffer.concat([checksum, hash]).toString('hex');
}
