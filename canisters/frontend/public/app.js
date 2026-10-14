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
import { ConfigurationMismatchError, registryMarkup, verifyInstanceConfiguration } from './instance-view.js';
import { renderRuntimeError } from './safe-dom.js';
import { isNeuronMode, normalizeMode, targetLabels } from './target-view.js';

import { createLedgerCounterClient } from './ledger-counter-client.js';
import { createLedgerCounter, formatTotal } from './ledger-counter.js';

const COMPACT_HERO_QUERY = '(max-width: 81.25rem), (max-aspect-ratio: 1/1)';

export function initHeroArtwork(documentRef, windowRef, schedule = callback => callback()) {
  const detail = documentRef.querySelector('.hero__detail');
  if (!detail) return { setPaused() {}, dispose() {} };
  let disposed = false, manuallyPaused = false;
  let windowFocused = documentRef.hasFocus?.() ?? true;
  const media = windowRef?.matchMedia?.('(prefers-reduced-motion: reduce)');
  const syncAnimationState = () => {
    if (disposed) return;
    const svg = detail.contentDocument?.documentElement;
    if (!svg) return;
    const paused = manuallyPaused || Boolean(media?.matches) || Boolean(documentRef.hidden) || !windowFocused;
    svg.classList.toggle('ehm-paused', paused);
    if (paused) svg.pauseAnimations?.();
    else svg.unpauseAnimations?.();
  };
  const reveal = () => {
    if (disposed) return;
    syncAnimationState();
    schedule(() => { if (!disposed) detail.classList.add('hero__detail--loaded'); });
  };
  const source = () => {
    const src = media?.matches ? detail.dataset.staticSrc : detail.dataset.animatedSrc;
    if (detail.getAttribute?.('data') !== src) detail.data = src;
    syncAnimationState();
  };
  const blur = () => { windowFocused = false; syncAnimationState(); };
  const focus = () => { windowFocused = true; syncAnimationState(); };
  detail.addEventListener('load', reveal);
  documentRef.addEventListener('visibilitychange', syncAnimationState);
  windowRef?.addEventListener?.('blur', blur);
  windowRef?.addEventListener?.('focus', focus);
  media?.addEventListener?.('change', source);
  const initialSrc = media?.matches ? detail.dataset.staticSrc : detail.dataset.animatedSrc;
  if (detail.getAttribute?.('data') !== initialSrc) detail.data = initialSrc;
  else if (detail.contentDocument?.documentElement?.namespaceURI === 'http://www.w3.org/2000/svg') reveal();
  return {
    setPaused(value) { manuallyPaused = value; syncAnimationState(); },
    dispose() {
      manuallyPaused = true; syncAnimationState(); disposed = true;
      detail.removeEventListener?.('load', reveal);
      documentRef.removeEventListener?.('visibilitychange', syncAnimationState);
      windowRef?.removeEventListener?.('blur', blur);
      windowRef?.removeEventListener?.('focus', focus);
      media?.removeEventListener?.('change', source);
    },
  };
}

export function initLedgerCounter(documentRef, windowRef, { query, artwork, dependencies = {} } = {}) {
  const node = id => documentRef.getElementById(id);
  if (!['ledger-count', 'ledger-label', 'ledger-sample', 'ledger-status', 'ledger-checked', 'live-effects'].every(id => node(id))
      || !windowRef?.AbortController || !windowRef?.requestAnimationFrame) return null;
  const write = (id, text) => { if (node(id).textContent !== text) node(id).textContent = text; };
  const client = query ? null : createLedgerCounterClient();
  let manual = false;
  const media = windowRef.matchMedia?.('(prefers-reduced-motion: reduce)');
  const controller = createLedgerCounter({
    query: query ?? client.query,
    render(state) {
      const icp = state.entry?.ledgerCountSource === 'icp-chain-length';
      const label = icp ? 'ICP ledger transactions' : `${state.entry?.name ?? 'Selected'} ledger records`;
      write('ledger-label', label);
      const text = state.displayed === null ? '—' : formatTotal(state.displayed);
      write('ledger-count', text);
      write('ledger-sample', state.accepted === null ? `${label}: unavailable`
        : `${label}: ${formatTotal(state.accepted)} (latest retrieved total)`);
      const messages = {
        loading: 'Checking selected ledger…', current: state.reduced ? 'Periodically refreshed' : 'Periodically refreshed · Smoothed display',
        unavailable: 'Ledger count unavailable · Retrying', unsupported: 'Ledger count unavailable for this instance',
        planned: 'Count unavailable until launch', paused: 'Live effects paused', hidden: 'Refresh suspended while hidden',
        retrying: 'Refresh failed · Last known total · Retrying', delayed: 'Data delayed · Last known total · Retrying',
        inconsistent: 'Inconsistent ledger responses · Last known total · Retrying',
        configuration: 'Count unavailable · Instance configuration mismatch',
      };
      write('ledger-status', messages[state.status]);
      write('ledger-checked', state.checked === null ? '' : `Last checked ${new Date(state.checked).toISOString()}`);
    },
    ...dependencies,
  });
  const lifecycle = () => controller.setLifecycle({ hidden: documentRef.hidden, paused: manual, reduced: media?.matches });
  const toggle = () => {
    manual = !manual;
    node('live-effects').textContent = manual ? 'Resume live effects' : 'Pause live effects';
    node('live-effects').setAttribute('aria-pressed', String(manual));
    artwork?.setPaused(manual); lifecycle();
  };
  node('live-effects').hidden = false;
  node('live-effects').addEventListener('click', toggle);
  documentRef.addEventListener('visibilitychange', lifecycle);
  media?.addEventListener?.('change', lifecycle);
  lifecycle();
  return {
    select: entry => controller.select(entry),
    configurationMismatch: () => controller.configurationMismatch(),
    dispose() {
      controller.dispose(); client?.dispose();
      node('live-effects').removeEventListener('click', toggle);
      documentRef.removeEventListener('visibilitychange', lifecycle);
      media?.removeEventListener?.('change', lifecycle);
    },
  };
}

export function initHeroBackdrop(documentRef, windowRef) {
  const art = documentRef.querySelector('.hero__art');
  if (!art || !windowRef) return;
  const supportsNativeFade = windowRef.CSS?.supports?.('animation-timeline: scroll()')
    && windowRef.CSS.supports('animation-range: 0 100svh');
  if (supportsNativeFade) return;
  const update = () => {
    const compact = windowRef.matchMedia(COMPACT_HERO_QUERY).matches;
    const fadeDistance = Math.max(
      compact ? windowRef.innerWidth * 941 / (1672 * .7) : windowRef.innerHeight,
      1,
    );
    const progress = Math.min(Math.max(windowRef.scrollY / fadeDistance, 0), 1);
    art.style.opacity = (1 - .86 * progress).toFixed(3);
  };
  update();
  windowRef.addEventListener('scroll', update, { passive: true });
  windowRef.addEventListener('resize', update, { passive: true });
}

export function createApp({
  documentRef,
  instances = INSTANCES,
  queryBackend = queryBackendPricing,
  autoLoad = true,
  counter = null,
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
    try { counter?.select(requested); } catch { /* Counter failure is isolated from the builder. */ }
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
      if (error instanceof ConfigurationMismatchError) {
        try { counter?.configurationMismatch(); } catch { /* Keep the builder fail closed. */ }
      }
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
  return { load, dispose() { ++loadGeneration; counter?.dispose(); $('memo-form').removeEventListener?.('input', render); $('instance').removeEventListener?.('change', load); } };
}

if (typeof document !== 'undefined') {
  const artwork = initHeroArtwork(document, window, callback => window.requestAnimationFrame(callback));
  initHeroBackdrop(document, window);
  let counter = null;
  try { counter = initLedgerCounter(document, window, { artwork }); } catch { /* Optional statistic. */ }
  const app = createApp({ documentRef: document, counter });
  window.addEventListener('pagehide', event => {
    if (!event.persisted) { app.dispose(); artwork.dispose(); }
  });
}
