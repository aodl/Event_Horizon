import { buildGlobalMemo, buildMemo, buildRangeMemo, compactPrincipal, memoByteLength } from './memo.js';
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
  const scope = $('scope').value;
  const global = scope === 'global';
  const range = scope === 'range';
  $('account-fields').hidden = global;
  $('single-field').hidden = global || range;
  $('range-fields').hidden = !range;
  try {
    const built = global
      ? buildGlobalMemo($('principal').value)
      : range
        ? buildRangeMemo($('principal').value, $('range-start').value, $('range-end').value, $('amount').value)
        : buildMemo($('principal').value, $('subaccount').value, $('amount').value);
    const { memo, bytes } = built;
    out.textContent = memo;
    const tier = global ? 'global_icp' : range ? 'range_icp' : 'account_icp';
    const current = pricing?.initialized ? pricing.current[tier] : null;
    const upcoming = pricing?.next ? pricing.next[tier] : null;
    const recommended = current === null ? null : Math.max(current, upcoming ?? current);
    help.textContent = `${bytes}/32 bytes · ${recommended === null ? 'Wait for authoritative pricing to initialize.' : `Endow this exact declaration with ${recommended} ICP through Jupiter Faucet.`}${upcoming > current ? ` The scheduled requirement rises from ${current} ICP at ${utc(pricing.next_effective_at)}; Faucet delivery is delayed, so the builder recommends ${upcoming} ICP now.` : ''}${!global && !$('amount').value.trim() ? ' Blank threshold means every incoming transfer is relevant.' : ''}`;
    help.classList.remove('error');
  } catch (e) {
    const principal = compactPrincipal($('principal').value);
    const account = range ? `${$('range-start').value}-${$('range-end').value}` : $('subaccount').value;
    const amount = $('amount').value.trim();
    const draft = global ? `X.${principal}` : `X.${principal}.${account}${amount ? `:${amount}` : ''}`;
    out.textContent = draft || '—';
    help.textContent = `${memoByteLength(draft)}/32 bytes · ${e.message}`;
    help.classList.add('error');
  }
}
form.addEventListener('input', render); render();
queryBackendPricing()
  .then(value => { pricing = value; renderPricing(); render(); })
  .catch(() => { $('pricing').textContent = 'Authoritative backend pricing is temporarily unavailable.'; render(); });
