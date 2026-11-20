export const INSTANCES = Object.freeze([
  Object.freeze({ id:'icp', name:'ICP', symbol:'ICP', alias:'E', status:'live', canonical:true, backendCanisterId:'eo6ei-gaaaa-aaaar-qchra-cai', observedLedgerCanisterId:'ryjl3-tyaaa-aaaaa-aaaba-cai', ledgerCountSource:'icp-chain-length', snsRootCanisterId:null, surplusCanisterId:null, icpAdmissionNotifierId:null, expectedBackendWasmSha256:'d889b6447273769776f4e5043bbfe73574dc6763225a62242a88da7927bd5e84' }),
  Object.freeze({ id:'io', name:'IO', symbol:'IO', alias:'I', status:'planned', canonical:true, backendCanisterId:null, observedLedgerCanisterId:null, ledgerCountSource:null, snsRootCanisterId:null, surplusCanisterId:null, icpAdmissionNotifierId:null, expectedBackendWasmSha256:null }),
]);
export const defaultInstance = () => INSTANCES[0];
