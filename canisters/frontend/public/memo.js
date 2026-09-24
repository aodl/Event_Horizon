export const MAX_JUPITER_MEMO_BYTES = 32;

export function compactPrincipal(text) {
  return text.trim().replaceAll('-', '');
}

export function isValidAmount(text) {
  if (!/^(0|[1-9][0-9]*)(\.[0-9]{1,2})?$/.test(text)) return false;
  const [whole, frac=''] = text.split('.');
  const cents = BigInt(whole) * 100n + BigInt((frac + '00').slice(0,2));
  return cents >= 1n; // explicit thresholds retain the 0.01 ICP floor
}

export function memoByteLength(memo) {
  return new TextEncoder().encode(memo).length;
}

function validatePrincipal(principalText) {
  const principal = compactPrincipal(principalText);
  if (!principal || !/^[a-z0-9]+$/i.test(principal)) throw new Error('Enter a canister principal.');
  return principal;
}

function numberedSubaccount(text) {
  if (!/^(0|[1-9][0-9]*)$/.test(text)) throw new Error('Subaccounts must use canonical decimal integers between 0 and 255.');
  const value = Number(text);
  if (!Number.isInteger(value) || value < 0 || value > 255) throw new Error('Subaccounts must be between 0 and 255.');
  return value;
}

function validateAmount(amountText) {
  const amount = amountText.trim();
  if (amount && !isValidAmount(amount)) {
    throw new Error('Trigger amount must be at least 0.01 ICP with at most two decimal places, or left blank for every transfer.');
  }
  return amount;
}

function checkedMemo(suffix, metadata = {}) {
  const memo = `X.${suffix}`;
  const bytes = memoByteLength(memo);
  if (bytes > MAX_JUPITER_MEMO_BYTES) throw new Error('This subscription memo exceeds the 32-byte Jupiter Faucet memo limit.');
  return { memo, bytes, ...metadata };
}

export function buildMemo(principalText, subaccountText, amountText = '') {
  const principal = validatePrincipal(principalText);
  const sub = numberedSubaccount(subaccountText);
  const amount = validateAmount(amountText);
  const suffix = amount ? `${principal}.${sub}:${amount}` : `${principal}.${sub}`;
  return checkedMemo(suffix, { thresholded: Boolean(amount), global: false, range: false });
}

export function buildRangeMemo(principalText, startText, endText, amountText = '') {
  const principal = validatePrincipal(principalText);
  const start = numberedSubaccount(startText);
  const end = numberedSubaccount(endText);
  if (end < start) throw new Error('Range end must be greater than range start.');
  if (end === start) throw new Error(`Use subaccount ${start} as a single-account subscription. A range must contain at least two subaccounts.`);
  const amount = validateAmount(amountText);
  const suffix = amount ? `${principal}.${start}-${end}:${amount}` : `${principal}.${start}-${end}`;
  return checkedMemo(suffix, { thresholded: Boolean(amount), global: false, range: true });
}

export function buildGlobalMemo(principalText) {
  return checkedMemo(validatePrincipal(principalText), { thresholded: false, global: true, range: false });
}
