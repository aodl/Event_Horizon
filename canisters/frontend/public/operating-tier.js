const T = 1_000_000_000_000n;
// Mirror the production entry table. The actual mode must come from the backend
// or an explicitly dated mode report; hysteresis makes balance-only inference wrong.
export const OPERATING_TIERS = Object.freeze([
  Object.freeze({ mode: 'ReserveProtection', label: 'Reserve protection', enter: 0n, delay: 'Ordinary polling suspended' }),
  Object.freeze({ mode: 'Economy', label: 'Economy', enter: 2n * T, delay: '1 hour added delay' }),
  Object.freeze({ mode: 'Standard', label: 'Standard', enter: 5n * T, delay: '10 minutes added delay' }),
  Object.freeze({ mode: 'Fast', label: 'Fast', enter: 10n * T, delay: '2 minutes added delay' }),
  Object.freeze({ mode: 'VeryFast', label: 'Very fast', enter: 25n * T, delay: '10 seconds added delay' }),
  Object.freeze({ mode: 'Continuous', label: 'Continuous', enter: 100n * T, delay: 'No added polling delay' }),
]);
export function formatCycles(value) {
  if (typeof value !== 'bigint' || value < 0n) throw new Error('Invalid cycles balance.');
  if (value > 0n && value < T / 1000n) return '<0.001 T';
  const whole = (value / T).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return `${whole}.${(value % T * 1000n / T).toString().padStart(3, '0')} T`;
}
export function tierProgress(mode, liquidCycles) {
  formatCycles(liquidCycles);
  const index = OPERATING_TIERS.findIndex(t => t.mode === mode);
  if (index < 0) throw new Error('Unknown operating tier.');
  const tier = OPERATING_TIERS[index], next = OPERATING_TIERS[index + 1] ?? null;
  if (!next) return { tier, next: null, remaining: 0n, percent: null };
  const remaining = liquidCycles < next.enter ? next.enter - liquidCycles : 0n;
  const percent = Number((liquidCycles < next.enter ? liquidCycles : next.enter) * 10000n / next.enter) / 100;
  return { tier, next, remaining, percent };
}
