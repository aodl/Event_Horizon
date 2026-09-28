export const INSTANCES = Object.freeze([
  Object.freeze({ id:'icp', name:'ICP', symbol:'ICP', alias:'X', status:'live', canonical:true, backendCanisterId:'eo6ei-gaaaa-aaaar-qchra-cai', observedLedgerCanisterId:'ryjl3-tyaaa-aaaaa-aaaba-cai', snsRootCanisterId:null, expectedBackendWasmSha256:'91a12fe2f63edb5f7a3bab311a34294195a45bdde8df6eae3f83d3694387ad77' }),
  Object.freeze({ id:'io', name:'IO', symbol:'IO', alias:'I', status:'planned', canonical:true, backendCanisterId:null, observedLedgerCanisterId:null, snsRootCanisterId:null, expectedBackendWasmSha256:null }),
]);
export const defaultInstance = () => INSTANCES[0];
