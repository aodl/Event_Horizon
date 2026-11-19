export const INSTANCES = Object.freeze([
  Object.freeze({ id:'icp', name:'ICP', symbol:'ICP', alias:'E', status:'live', canonical:true, backendCanisterId:'eo6ei-gaaaa-aaaar-qchra-cai', observedLedgerCanisterId:'ryjl3-tyaaa-aaaaa-aaaba-cai', ledgerCountSource:'icp-chain-length', snsRootCanisterId:null, surplusCanisterId:null, icpAdmissionNotifierId:null, expectedBackendWasmSha256:'246e639cef13ecbd39d23b240924609fc12493381f7332ed969f7ca6b11f4821' }),
  Object.freeze({ id:'io', name:'IO', symbol:'IO', alias:'I', status:'planned', canonical:true, backendCanisterId:null, observedLedgerCanisterId:null, ledgerCountSource:null, snsRootCanisterId:null, surplusCanisterId:null, icpAdmissionNotifierId:null, expectedBackendWasmSha256:null }),
]);
export const defaultInstance = () => INSTANCES[0];
