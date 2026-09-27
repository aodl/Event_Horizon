export const MAX_JUPITER_MEMO_BYTES = 32;
export const U64_MAX = 18446744073709551615n;

export function compactPrincipal(text) {
  return text.trim().replaceAll('-', '');
}

export function isValidAmount(text, decimals) {
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 255) return false;
  if (!/^(0|[1-9][0-9]*)(\.[0-9]+)?$/.test(text)) return false;
  const [whole, frac=''] = text.split('.');
  if (frac.length > decimals) return false;
  const units = BigInt(whole) * (10n ** BigInt(decimals)) + BigInt((frac + '0'.repeat(decimals)).slice(0, decimals) || '0');
  return units > 0n;
}

export function memoByteLength(memo) {
  return new TextEncoder().encode(memo).length;
}

export function validatePrincipal(principalText) {
  const trimmed = principalText.trim();
  let parsed;
  try { parsed = Principal.fromText(trimmed); } catch { throw new Error('Enter a valid canister principal.'); }
  if (parsed.isAnonymous()) throw new Error('The anonymous principal cannot subscribe.');
  if (parsed.toText() === Principal.managementCanister().toText()) throw new Error('The management principal cannot subscribe.');
  // Rust Principal::from_text accepts this checksum-validated compact form; it is
  // the established Jupiter memo representation and saves bytes under the limit.
  return compactPrincipal(parsed.toText());
}

function numberedSubaccount(text) {
  if (!/^(0|[1-9][0-9]*)$/.test(text)) throw new Error('Targets must use canonical unsigned decimal integers.');
  const value = BigInt(text);
  if (value > U64_MAX) throw new Error('Target exceeds the maximum unsigned 64-bit value.');
  return { value, text };
}

function validateAmount(amountText, decimals) {
  const amount = amountText.trim();
  if (amount && !isValidAmount(amount, decimals)) {
    throw new Error(`Trigger amount must be positive with at most ${decimals} decimal places, or left blank for every transfer.`);
  }
  return amount;
}

function checkedMemo(alias, suffix, metadata = {}) {
  if (!/^[\x21-\x7e]+$/.test(alias)) throw new Error('Instance alias must be explicit printable ASCII.');
  const memo = `${alias}.${suffix}`;
  const bytes = memoByteLength(memo);
  if (bytes > MAX_JUPITER_MEMO_BYTES) throw new Error('This subscription memo exceeds the 32-byte Jupiter Faucet memo limit.');
  return { memo, bytes, ...metadata };
}

export function buildMemo(alias, decimals, principalText, subaccountText, amountText = '') {
  const principal = validatePrincipal(principalText);
  const sub = numberedSubaccount(subaccountText).text;
  const amount = validateAmount(amountText, decimals);
  const suffix = amount ? `${principal}.${sub}:${amount}` : `${principal}.${sub}`;
  return checkedMemo(alias, suffix, { thresholded: Boolean(amount), global: false, range: false });
}

export function buildRangeMemo(alias, decimals, principalText, startText, endText, amountText = '') {
  const principal = validatePrincipal(principalText);
  const start = numberedSubaccount(startText), end = numberedSubaccount(endText);
  if (end.value < start.value) throw new Error('Range end must be greater than range start.');
  if (end.value === start.value) throw new Error(`Use target ${start.text} as a single-target subscription. A range must contain at least two targets.`);
  if (end.value - start.value > 255n) throw new Error('A range may contain at most 256 targets.');
  const amount = validateAmount(amountText, decimals);
  const suffix = amount ? `${principal}.${start.text}-${end.text}:${amount}` : `${principal}.${start.text}-${end.text}`;
  return checkedMemo(alias, suffix, { thresholded: Boolean(amount), global: false, range: true });
}

export function buildNeuronMemo(alias, decimals, principalText, nonceText, amountText = '') {
  const principal = validatePrincipal(principalText), nonce = numberedSubaccount(nonceText).text;
  const amount = validateAmount(amountText, decimals);
  return checkedMemo(alias, amount ? `${principal}.n${nonce}:${amount}` : `${principal}.n${nonce}`, { thresholded:Boolean(amount), global:false, range:false, neuron:true });
}

export function buildNeuronRangeMemo(alias, decimals, principalText, startText, endText, amountText = '') {
  const principal = validatePrincipal(principalText), start=numberedSubaccount(startText), end=numberedSubaccount(endText);
  if (end.value <= start.value) throw new Error('Range end must be greater than range start.');
  if (end.value - start.value > 255n) throw new Error('A range may contain at most 256 targets.');
  const amount=validateAmount(amountText,decimals);
  return checkedMemo(alias, amount ? `${principal}.n${start.text}-${end.text}:${amount}` : `${principal}.n${start.text}-${end.text}`, { thresholded:Boolean(amount),global:false,range:true,neuron:true });
}

export function buildGlobalMemo(alias, principalText) {
  return checkedMemo(alias, validatePrincipal(principalText), { thresholded: false, global: true, range: false });
}
import { Principal } from '@icp-sdk/core/principal';
