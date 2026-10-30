export const INSTANCES = Object.freeze([
  Object.freeze({ id:'icp', name:'ICP', symbol:'ICP', alias:'E', status:'live', canonical:true, backendCanisterId:'eo6ei-gaaaa-aaaar-qchra-cai', observedLedgerCanisterId:'ryjl3-tyaaa-aaaaa-aaaba-cai', ledgerCountSource:'icp-chain-length', snsRootCanisterId:null, surplusCanisterId:null, expectedBackendWasmSha256:'f93cd920a6a7e147462d5a359fa33a4c1bbae7677bbe2de6242b01ac690f8970' }),
  Object.freeze({ id:'io', name:'IO', symbol:'IO', alias:'I', status:'planned', canonical:true, backendCanisterId:null, observedLedgerCanisterId:null, ledgerCountSource:null, snsRootCanisterId:null, surplusCanisterId:null, expectedBackendWasmSha256:null }),
]);
export const defaultInstance = () => INSTANCES[0];
