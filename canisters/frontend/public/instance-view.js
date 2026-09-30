const optionalPrincipal = value => value.length ? value[0].toText() : null;

export function verifyInstanceConfiguration(registry, instance) {
  const observedLedger = instance.observed_ledger.toText();
  const snsRoot = optionalPrincipal(instance.sns_root);
  const surplusCanister = optionalPrincipal(instance.surplus_canister);
  if (observedLedger !== registry.observedLedgerCanisterId
      || snsRoot !== registry.snsRootCanisterId
      || surplusCanister !== registry.surplusCanisterId) {
    throw new Error(
      `Configuration mismatch: backend reports observed ledger ${observedLedger}, `
      + `SNS Root ${snsRoot ?? 'none'}, and surplus recipient ${surplusCanister ?? 'none'}.`,
    );
  }
  if (!instance.observed_profile.length) {
    throw new Error('Configuration mismatch: backend observed-ledger profile is not initialized.');
  }
  const profile = instance.observed_profile[0];
  if (profile.symbol !== registry.symbol) {
    throw new Error(`Configuration mismatch: backend symbol is ${profile.symbol}.`);
  }
  return profile;
}

export function registryMarkup(instance, profile = null) {
  const governance = profile?.neuron_governance?.length
    ? profile.neuron_governance[0].toText()
    : 'none';
  return `<strong>${instance.name} — ${instance.status === 'live' ? 'live' : 'coming soon'}</strong><br>`
    + `Alias: ${instance.alias}<br>`
    + `Backend: ${instance.backendCanisterId ?? 'pending'}<br>`
    + `Observed ledger: ${instance.observedLedgerCanisterId ?? 'pending'}<br>`
    + `SNS Root: ${instance.snsRootCanisterId ?? 'none'}<br>`
    + `Surplus recipient: ${instance.surplusCanisterId ?? 'none'}<br>`
    + `Neuron Governance: ${profile ? governance : 'pending'}<br>`
    + `Expected backend Wasm SHA-256: ${instance.expectedBackendWasmSha256 ?? 'pending'}<br>`
    + `${instance.canonical ? 'Canonical' : 'Community'} listing`;
}
