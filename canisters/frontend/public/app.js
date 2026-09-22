import { buildGlobalMemo, buildMemo } from './memo.js';
const $ = id => document.getElementById(id);
const form = $('memo-form');
let pricing = null;
const utc = seconds => seconds ? new Date(seconds * 1000).toISOString().replace('.000Z', ' UTC') : 'pending';
function renderPricing() {
  const out = $('pricing');
  if (!pricing?.initialized) {
    out.textContent = 'Pricing is initializing from Event Horizon’s first successful CMC observation.';
    return;
  }
  const next = pricing.next;
  out.innerHTML = `<p><strong>Current requirement:</strong> ${pricing.current.account_icp} ICP per account declaration · ${pricing.current.global_icp} ICP per global declaration</p>
    <p>Current period began ${utc(pricing.current_effective_at)}. Next boundary: ${utc(pricing.next_effective_at)}. Price freeze: ${utc(pricing.next_freeze_at)}.</p>
    ${next ? `<p><strong>Scheduled from ${utc(pricing.next_effective_at)}:</strong> ${next.account_icp} ICP account · ${next.global_icp} ICP global${pricing.next_carried_forward_due_to_stale_rate ? ' (current prices carried forward because the latest rate was stale)' : ''}</p>` : '<p>The next price has not been frozen yet.</p>'}
    <p>Recorded four-year floor: ${pricing.observed_floor_xdr_permyriad} XDR permyriad at ${utc(pricing.floor_observed_at)}. Latest: ${pricing.latest_xdr_permyriad} at ${utc(pricing.latest_observed_at)}.</p>`;
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
fetch('/pricing.json', { cache: 'no-store' })
  .then(response => { if (!response.ok) throw new Error(); return response.json(); })
  .then(value => { pricing = value; renderPricing(); render(); })
  .catch(() => { $('pricing').textContent = 'Authoritative pricing is temporarily unavailable.'; render(); });
