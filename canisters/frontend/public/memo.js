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

export function buildMemo(principalText, subaccountText, amountText = '') {
  const principal = compactPrincipal(principalText);
  if (!principal || !/^[a-z0-9]+$/i.test(principal)) throw new Error('Enter a canister principal.');
  if (!/^[0-9]+$/.test(subaccountText)) throw new Error('Subaccount must be 0–255.');
  const sub = Number(subaccountText);
  if (!Number.isInteger(sub) || sub < 0 || sub > 255) throw new Error('Subaccount must be 0–255.');

  const amount = amountText.trim();
  if (amount && !isValidAmount(amount)) {
    throw new Error('Trigger amount must be at least 0.01 ICP with at most two decimal places, or left blank for every transfer.');
  }

  const suffix = amount ? `${principal}.${sub}:${amount}` : `${principal}.${sub}`;
  const memo = `X.${suffix}`;
  const bytes = new TextEncoder().encode(memo).length;
  if (bytes > MAX_JUPITER_MEMO_BYTES) throw new Error(`Memo is ${bytes} bytes; Jupiter Faucet allows ${MAX_JUPITER_MEMO_BYTES}.`);
  return { memo, bytes, thresholded: Boolean(amount) };
}

export function buildGlobalMemo(principalText) {
  const principal = compactPrincipal(principalText);
  if (!principal || !/^[a-z0-9]+$/i.test(principal)) throw new Error('Enter a canister principal.');
  const memo = `X.${principal}`;
  const bytes = new TextEncoder().encode(memo).length;
  if (bytes > MAX_JUPITER_MEMO_BYTES) throw new Error(`Memo is ${bytes} bytes; Jupiter Faucet allows ${MAX_JUPITER_MEMO_BYTES}.`);
  return { memo, bytes, thresholded: false, global: true };
}
