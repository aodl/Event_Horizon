export const INSTANCES = Object.freeze([
  Object.freeze({ id:'icp', name:'ICP', symbol:'ICP', alias:'E', status:'live', canonical:true, backendCanisterId:'eo6ei-gaaaa-aaaar-qchra-cai', observedLedgerCanisterId:'ryjl3-tyaaa-aaaaa-aaaba-cai', ledgerCountSource:'icp-chain-length', snsRootCanisterId:null, surplusCanisterId:null, icpAdmissionNotifierId:null, expectedBackendWasmSha256:'b861ad08971a2539eb244a8d22a93ae2d527fcefe2c192f88bea6c00f715e9ec' }),
  Object.freeze({ id:'io', name:'IO', symbol:'IO', alias:'I', status:'planned', canonical:true, backendCanisterId:null, observedLedgerCanisterId:null, ledgerCountSource:null, snsRootCanisterId:null, surplusCanisterId:null, icpAdmissionNotifierId:null, expectedBackendWasmSha256:null }),
]);
export const defaultInstance = () => INSTANCES[0];
