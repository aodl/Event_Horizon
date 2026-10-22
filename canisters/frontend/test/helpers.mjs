export const deferred=()=>{let resolve,reject;const promise=new Promise((ok,no)=>{resolve=ok;reject=no});return{promise,resolve,reject}};
export const principal=text=>({toText:()=>text});
export const liveEntry=(id,backend)=>({id,name:id.toUpperCase(),symbol:'TOK',alias:id.toUpperCase(),status:'live',canonical:true,backendCanisterId:backend,observedLedgerCanisterId:'ledger',snsRootCanisterId:null,surplusCanisterId:null,expectedBackendWasmSha256:'hash'});
export const backendResult=(entry,accountIcp,{rangeIcp=accountIcp*2,globalIcp=accountIcp*100,next=null}={})=>({instance:{observed_ledger:principal(entry.observedLedgerCanisterId),sns_root:[],surplus_canister:[],observed_profile:[{symbol:entry.symbol,decimals:8,neuron_governance:[]}]},pricing:{initialized:true,current:{account_icp:accountIcp,range_icp:rangeIcp,global_icp:globalIcp},current_effective_at:1,next,next_effective_at:2,next_freeze_at:3,observed_floor_xdr_permyriad:10000,floor_observed_at:1,latest_xdr_permyriad:10000,latest_observed_at:1,next_carried_forward_due_to_stale_rate:false}});

export function eventTarget() {
  const listeners = new Map();
  return {
    listeners,
    addEventListener(name, fn) { if (!listeners.has(name)) listeners.set(name, new Set()); listeners.get(name).add(fn); },
    removeEventListener(name, fn) { listeners.get(name)?.delete(fn); },
    dispatch(name, event = {}) { for (const fn of [...(listeners.get(name) ?? [])]) fn(event); },
  };
}
export function fakeDocument({ counter = false } = {}) {
  const makeNode = () => ({ ...eventTarget(), textContent: '', innerHTML: '', value: '', disabled: false, hidden: false,
    attributes: {}, setAttribute(name, value) { this.attributes[name] = value; },
    classList: { values: new Set(), add(value) { this.values.add(value); }, remove(value) { this.values.delete(value); } },
    append(option) { this.options ??= []; this.options.push(option); if (!this.value) this.value = option.value; },
  });
  const ids = ['instance', 'instance-info', 'pricing', 'scope', 'single-label', 'range-start-label', 'range-end-label', 'account-fields', 'single-field', 'range-fields', 'memo-output', 'memo-help', 'subject-mode', 'subject-field', 'subject', 'principal', 'subaccount', 'range-start', 'range-end', 'amount'];
  if (counter) ids.push('ledger-rate', 'ledger-rate-description', 'ledger-count', 'ledger-label', 'ledger-sample', 'ledger-status', 'ledger-checked');
  const nodes = Object.fromEntries(ids.map(id => [id, makeNode()]));
  nodes.scope.value = 'account';
  nodes['subject-mode'].value='self';
  const scopeOptions = { neuron: makeNode(), 'neuron-range': makeNode() };
  nodes.scope.querySelector = selector => scopeOptions[selector.match(/value="([^"]+)"/)[1]];
  nodes['amount-label'] = makeNode(); nodes['amount-label'].firstChild = { textContent: '' };
  nodes['memo-form'] = makeNode();
  nodes['memo-form'].elements = [nodes.scope, nodes['subject-mode'], nodes.subject, nodes.principal, nodes.subaccount, nodes['range-start'], nodes['range-end'], nodes.amount];
  const events = eventTarget();
  return { ...events, nodes, hidden: false, getElementById: id => nodes[id], createElement: () => makeNode(),
    dispatch(type, event) { if (type === 'input') nodes['memo-form'].dispatch(type, event); else if (type === 'change') nodes.instance.dispatch(type, event); else events.dispatch(type, event); },
  };
}
