import {
  buildGlobalMemo,
  buildMemo,
  buildRangeMemo,
  buildNeuronMemo,
  buildNeuronRangeMemo,
} from './memo.js';
import { queryBackendPricing } from './pricing-client.js';
import { pricingMarkup, utc } from './pricing-view.js';
import { INSTANCES } from './instances.js';
import { registryMarkup, verifyInstanceConfiguration } from './instance-view.js';
import { renderRuntimeError } from './safe-dom.js';
import { isNeuronMode, normalizeMode, targetLabels } from './target-view.js';

export function createApp({
  documentRef,
  instances = INSTANCES,
  queryBackend = queryBackendPricing,
  autoLoad = true,
} = {}) {
  const doc = documentRef ?? document;
  const $ = id => doc.getElementById(id);
  let selected = instances[0];
  let pricing = null;
  let profile = null;
  let verified = false;
  let loadGeneration = 0;

  function setDisabled() {
    const disabled = !verified;
    for (const input of $('memo-form').elements) input.disabled = disabled;
    const capable = Boolean(profile?.neuron_governance?.length);
    for (const value of ['neuron', 'neuron-range']) {
      $('scope').querySelector(`option[value="${value}"]`).disabled = disabled || !capable;
    }
    $('scope').value = normalizeMode($('scope').value, capable);
    $('instance').disabled = false;
  }

  function render() {
    const mode = $('scope').value;
    const global = mode === 'global';
    const range = mode === 'range' || mode === 'neuron-range';
    const neuron = isNeuronMode(mode);
    const labels = targetLabels(mode);
    $('single-label').textContent = labels.single;
    $('range-start-label').textContent = labels.start;
    $('range-end-label').textContent = labels.end;
    $('account-fields').hidden = global;
    $('single-field').hidden = global || range;
    $('range-fields').hidden = !range;
    $('amount-label').firstChild.textContent = `Minimum ${selected.symbol} amount (optional) `;
    const out = $('memo-output');
    const help = $('memo-help');
    if (!verified) {
      out.textContent = '—';
      help.textContent = selected.status === 'planned'
        ? 'This canonical instance is planned; backend and ledger IDs are pending.'
        : 'Configuration verification is required before constructing a subscription.';
      help.classList.add('error');
      return;
    }
    if (neuron && !profile.neuron_governance.length) {
      out.textContent = '—';
      help.textContent = 'This instance does not support neuron targets.';
      help.classList.add('error');
      return;
    }
    try {
      const args = [selected.alias, profile.decimals, $('principal').value];
      const amount = $('amount').value;
      const built = global
        ? buildGlobalMemo(selected.alias, $('principal').value)
        : mode === 'range'
          ? buildRangeMemo(...args, $('range-start').value, $('range-end').value, amount)
          : mode === 'neuron-range'
            ? buildNeuronRangeMemo(...args, $('range-start').value, $('range-end').value, amount)
            : mode === 'neuron'
              ? buildNeuronMemo(...args, $('subaccount').value, amount)
              : buildMemo(...args, $('subaccount').value, amount);
      out.textContent = built.memo;
      const tier = global ? 'global_icp' : range ? 'range_icp' : 'account_icp';
      const current = pricing?.initialized ? pricing.current[tier] : null;
      const upcoming = pricing?.next ? pricing.next[tier] : null;
      const recommended = current === null ? null : Math.max(current, upcoming ?? current);
      help.textContent = `${built.bytes}/32 bytes · ${recommended === null
        ? 'Wait for authoritative pricing to initialize.'
        : `Endow this exact declaration with ${recommended} ICP through Jupiter Faucet.`}${upcoming > current
        ? ` Scheduled requirement rises at ${utc(pricing.next_effective_at)}.`
        : ''}`;
      help.classList.remove('error');
    } catch (error) {
      out.textContent = '—';
      help.textContent = error.message;
      help.classList.add('error');
    }
  }

  async function load() {
    const generation = ++loadGeneration;
    const requested = instances.find(instance => instance.id === $('instance').value);
    selected = requested;
    $('instance-info').innerHTML = registryMarkup(requested);
    pricing = null;
    profile = null;
    verified = false;
    setDisabled();
    $('pricing').textContent = requested.status === 'planned'
      ? 'Pricing unavailable until launch.'
      : 'Verifying backend configuration…';
    if (requested.status !== 'live') {
      render();
      return;
    }
    try {
      const result = await queryBackend({ canisterId: requested.backendCanisterId });
      if (generation !== loadGeneration) return;
      const verifiedProfile = verifyInstanceConfiguration(requested, result.instance);
      if (generation !== loadGeneration) return;
      profile = verifiedProfile;
      pricing = result.pricing;
      verified = true;
      $('instance-info').innerHTML = registryMarkup(requested, verifiedProfile);
      $('pricing').innerHTML = pricingMarkup(result.pricing);
    } catch (error) {
      if (generation !== loadGeneration) return;
      renderRuntimeError($('pricing'), error.message);
    }
    if (generation !== loadGeneration) return;
    setDisabled();
    render();
  }

  for (const instance of instances) {
    const option = doc.createElement('option');
    option.value = instance.id;
    option.textContent = `${instance.name}${instance.status === 'planned' ? ' — coming soon' : ''}`;
    $('instance').append(option);
  }
  $('memo-form').addEventListener('input', render);
  $('instance').addEventListener('change', load);
  if (autoLoad) load();
  return { load };
}

if (typeof document !== 'undefined') createApp({ documentRef: document });
