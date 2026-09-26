export const INSTANCES = Object.freeze([
  Object.freeze({ id:'icp', name:'ICP', symbol:'ICP', alias:'X', status:'live', canonical:true, backendCanisterId:'eo6ei-gaaaa-aaaar-qchra-cai', observedLedgerCanisterId:'ryjl3-tyaaa-aaaaa-aaaba-cai', expectedBackendWasmSha256:'b6588d0030e7cbab81b79d67827c40669d5eb1a9a30b03c5c54e7f134d0036f6' }),
  Object.freeze({ id:'io', name:'IO', symbol:'IO', alias:'I', status:'planned', canonical:true, backendCanisterId:null, observedLedgerCanisterId:null, expectedBackendWasmSha256:null }),
]);
export const defaultInstance = () => INSTANCES[0];
