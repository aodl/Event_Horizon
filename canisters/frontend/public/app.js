import { buildGlobalMemo, buildMemo } from './memo.js';
import { queryBackendPricing } from './pricing-client.js';
import { pricingMarkup, utc } from './pricing-view.js';

const $ = id => document.getElementById(id);
const form = $('memo-form');
let pricing = null;
function renderPricing() {
  $('pricing').innerHTML = pricingMarkup(pricing);
}
function render() {
  const out = $('memo-output'); const help = $('memo-help');
  try {
    const global = $('scope').value === 'global';
    $('account-fields').hidden = global;
    const { memo, bytes } = global
      ? buildGlobalMemo($('principal').value)
      : buildMemo($('principal').value, $('subaccount').value, $('amount').value);
    out.textContent = memo;
    const current = pricing?.initialized ? (global ? pricing.current.global_icp : pricing.current.account_icp) : null;
    const upcoming = pricing?.next ? (global ? pricing.next.global_icp : pricing.next.account_icp) : null;
    const recommended = current === null ? null : Math.max(current, upcoming ?? current);
    help.textContent = `${bytes}/32 bytes · ${recommended === null ? 'Wait for authoritative pricing to initialize.' : `Endow this exact declaration with ${recommended} ICP through Jupiter Faucet.`}${upcoming > current ? ` The scheduled requirement rises from ${current} ICP at ${utc(pricing.next_effective_at)}; Faucet delivery is delayed, so the builder recommends ${upcoming} ICP now.` : ''}${!global && !$('amount').value.trim() ? ' Blank threshold means every incoming transfer is relevant.' : ''}`;
    help.classList.remove('error');
  } catch (e) {
    out.textContent = '—'; help.textContent = e.message; help.classList.add('error');
  }
}
form.addEventListener('input', render); render();
queryBackendPricing()
  .then(value => { pricing = value; renderPricing(); render(); })
  .catch(() => { $('pricing').textContent = 'Authoritative backend pricing is temporarily unavailable.'; render(); });
