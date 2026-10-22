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

function parsePrincipal(text) {
  return Principal.fromText(text.includes('-')?text:text.match(/.{1,5}/g)?.join('-')??'');
}

export function validatePrincipal(principalText) {
  const trimmed = principalText.trim();
  let parsed;
  try { parsed = parsePrincipal(trimmed); } catch { throw new Error('Enter a valid canister principal.'); }
  if (parsed.toUint8Array().at(-1)!==1) throw new Error('Enter a canister principal, not a user identity.');
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

export function validateReference(text) {
  if (!text || text !== text.trim() || /\s/.test(text)) throw new Error('Enter a canister principal or registered uppercase alias without whitespace.');
  if (/^[A-Z][A-Z0-9]{0,7}$/.test(text)) {
    if (text === 'X') throw new Error('X is retired. Canonical Event Horizon routing uses E.');
    return { text, alias: true };
  }
  return { text: validatePrincipal(text), alias: false };
}
export function buildSubscriptionMemo({routingAlias,decimals,subscriber,subject='',mode='subaccount',start='0',end='',amount='',neuronGovernance=false}) {
  if (!['account','subaccount','range','neuron','neuron-range','global'].includes(mode)) throw new Error('Unknown subscription mode.');
  if (!/^[A-Z][A-Z0-9]{0,7}$/.test(routingAlias)) throw new Error('Invalid reviewed routing alias.');
  if (amount!==amount.trim()) throw new Error('Trigger amount must not contain surrounding whitespace.');
  if (routingAlias==='X') throw new Error('X is retired; use the reviewed routing alias.');
  const sub=validateReference(subscriber), other=subject?validateReference(subject):null;
  if (mode==='global' && other) throw new Error('Global subscriptions cannot select another subject.');
  if (mode.startsWith('neuron') && !neuronGovernance) throw new Error('Verified neuron governance is required.');
  const refs=other?`${sub.text}@${other.text}`:sub.text;
  let suffix, count=0;
  if (mode==='global') suffix=refs;
  else {
    const first=numberedSubaccount(start);const range=mode==='range'||mode==='neuron-range';
    let targets=first.text;count=1;
    if (range) {const last=numberedSubaccount(end);if(last.value<=first.value)throw new Error('A range needs two increasing endpoints.');const width=last.value-first.value+1n;if(width>256n)throw new Error(`A range contains ${width} targets; at most 256 targets are allowed.`);count=Number(width);targets+=`-${last.text}`;}
    if (mode.startsWith('neuron')) targets=`n${targets}`;
    const threshold=validateAmount(amount,decimals);suffix=`${refs}.${targets}${threshold?`:${threshold}`:''}`;
  }
  const result=checkedMemo(routingAlias,suffix,{targetCount:count,needsPreview:sub.alias||Boolean(other?.alias),suffix});
  return {...result,subscriber:sub.text,subject:other?.text??sub.text,subscriberPrincipal:sub.alias?null:parsePrincipal(sub.text).toText(),subjectPrincipal:(other??sub).alias?null:parsePrincipal((other??sub).text).toText()};
}
