export const INSTANCES = Object.freeze([
  Object.freeze({ id:'icp', name:'ICP', symbol:'ICP', alias:'E', status:'live', canonical:true, backendCanisterId:'eo6ei-gaaaa-aaaar-qchra-cai', observedLedgerCanisterId:'ryjl3-tyaaa-aaaaa-aaaba-cai', ledgerCountSource:'icp-chain-length', snsRootCanisterId:null, surplusCanisterId:null, expectedBackendWasmSha256:'2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528' }),
  Object.freeze({ id:'io', name:'IO', symbol:'IO', alias:'I', status:'planned', canonical:true, backendCanisterId:null, observedLedgerCanisterId:null, ledgerCountSource:null, snsRootCanisterId:null, surplusCanisterId:null, expectedBackendWasmSha256:null }),
]);
export const defaultInstance = () => INSTANCES[0];
