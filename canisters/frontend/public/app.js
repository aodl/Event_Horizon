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

const COMPACT_HERO_QUERY = '(max-width: 81.25rem), (max-aspect-ratio: 1/1)';

export function initHeroArtwork(documentRef, windowRef, schedule = callback => callback()) {
  const detail = documentRef.querySelector('.hero__detail');
  if (!detail) return;
  let loaded = false;
  let windowFocused = documentRef.hasFocus?.() ?? true;
  const syncAnimationState = () => {
    if (!loaded) return;
    const svg = detail.contentDocument?.documentElement;
    if (!svg) return;
    const paused = Boolean(documentRef.hidden) || !windowFocused;
    svg.classList.toggle('ehm-paused', paused);
    if (paused) svg.pauseAnimations?.();
    else svg.unpauseAnimations?.();
  };
  const reveal = () => {
    if (loaded) return;
    loaded = true;
    syncAnimationState();
    schedule(() => detail.classList.add('hero__detail--loaded'));
  };
  detail.addEventListener('load', reveal, { once: true });
  documentRef.addEventListener('visibilitychange', syncAnimationState);
  windowRef?.addEventListener?.('blur', () => {
    windowFocused = false;
    syncAnimationState();
  });
  windowRef?.addEventListener?.('focus', () => {
    windowFocused = true;
    syncAnimationState();
  });
  const reducedMotion = windowRef?.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  const selectedSrc = reducedMotion ? detail.dataset.staticSrc : detail.dataset.animatedSrc;
  const sourceChanged = detail.getAttribute?.('data') !== selectedSrc;
  if (sourceChanged) detail.data = selectedSrc;
  else if (detail.contentDocument?.documentElement?.namespaceURI === 'http://www.w3.org/2000/svg') {
    reveal();
  }
}

export function initHeroBackdrop(documentRef, windowRef) {
  const art = documentRef.querySelector('.hero__art');
  if (!art || !windowRef) return;
  let queued = false;
  const update = () => {
    queued = false;
    if (!windowRef.matchMedia(COMPACT_HERO_QUERY).matches) {
      art.style.removeProperty('--hero-art-scroll-opacity');
      return;
    }
    const fadeDistance = Math.max(windowRef.innerWidth * 941 / (1672 * .7), 1);
    const progress = Math.min(Math.max(windowRef.scrollY / fadeDistance, 0), 1);
    const eased = progress * progress * (3 - 2 * progress);
    art.style.setProperty('--hero-art-scroll-opacity', (1 - .86 * eased).toFixed(3));
  };
  const queueUpdate = () => {
    if (queued) return;
    queued = true;
    windowRef.requestAnimationFrame(update);
  };
  update();
  windowRef.addEventListener('scroll', queueUpdate, { passive: true });
  windowRef.addEventListener('resize', queueUpdate, { passive: true });
}

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
        : `Minimum qualifying cumulative endowment: ${recommended} ICP through Jupiter Faucet.`}${upcoming > current
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
    render();
    if (requested.status !== 'live') {
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

if (typeof document !== 'undefined') {
  initHeroArtwork(document, window, callback => window.requestAnimationFrame(callback));
  initHeroBackdrop(document, window);
  createApp({ documentRef: document });
}
