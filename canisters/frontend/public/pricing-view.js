export const utc = seconds => seconds ? new Date(seconds * 1000).toISOString().replace('.000Z', ' UTC') : 'pending';

export function formatXdrPermyriad(value) {
  const rate = BigInt(value);
  const whole = rate / 10_000n;
  const fraction = (rate % 10_000n).toString().padStart(4, '0');
  return `${whole}.${fraction} XDR/ICP`;
}

export function pricingMarkup(pricing) {
  if (!pricing?.initialized) return 'Pricing is initializing from Event Horizon’s first successful CMC observation.';
  const next = pricing.next;
  return `<p><strong>Current requirement:</strong> ${pricing.current.account_icp} ICP per account declaration · ${pricing.current.global_icp} ICP per global declaration</p>
    <p>Current period began ${utc(pricing.current_effective_at)}. Next boundary: ${utc(pricing.next_effective_at)}. Price freeze: ${utc(pricing.next_freeze_at)}.</p>
    ${next ? `<p><strong>Scheduled from ${utc(pricing.next_effective_at)}:</strong> ${next.account_icp} ICP account · ${next.global_icp} ICP global${pricing.next_carried_forward_due_to_stale_rate ? ' (current prices carried forward because the latest rate was stale)' : ''}</p>` : '<p>The next price has not been frozen yet.</p>'}
    <p>Event Horizon’s recorded CMC floor observation: ${formatXdrPermyriad(pricing.observed_floor_xdr_permyriad)} at ${utc(pricing.floor_observed_at)}. Latest recorded CMC observation: ${formatXdrPermyriad(pricing.latest_xdr_permyriad)} at ${utc(pricing.latest_observed_at)}.</p>`;
}
