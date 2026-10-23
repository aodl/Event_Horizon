# Index wake-up interoperability and callback documentation cleanup

Reviewed baseline: `3e24af80a6dc14dab2917488a605fe06a4cc8a2c`.

This follow-up preserves the normative `poke(PokeNotification)` sender contract,
production exports, storage and funding semantics. The proposed Index fixture
is test-only and genuinely exports `poke : () -> ()`, authenticating the
configured Event Horizon principal. It cannot inspect notification fields.
No mainnet deployment or Jupiter changes are authorized or performed.

## Fixture and real callback results

The separate `mock-index-poke` crate exports:

```candid
service : (principal) -> {
  poke : () -> ();
  debug_pokes : () -> (nat64) query;
  debug_last_caller : () -> (opt principal) query;
}
```

Its constructor records the trusted Event Horizon principal. The ordinary
zero-argument update method authenticates its caller before recording a wake.
A native exported-Candid assertion verifies the genuine zero-argument signature;
all positive delivery tests invoke Event Horizon polling, never the Index's
`poke()` directly from the driver.

Both canonical ICP and generic ICRC-3 global-only cases passed: ICP/Jupiter
funding admits the Index, ten unrelated observed-ledger blocks coalesce into
one attempted/accepted oneway call and one receiver execution, the caller is
Event Horizon, and a following empty poll emits nothing. In the generic case,
ICP-only activity after admission produces no wake; only the configured ICRC-3
observed ledger does. The receiver has no alternate callback method, and sender
attempt counters establish no fallback or second call.

The generic mixed-payload case pairs the Index with a full-record receiver
watching the same Index-owned and external accounts. The latter verifies the
entire decoded/re-encoded notification by digest. Each poll attempts exactly
one call per receiver; the subsequent empty poll emits none.

| Global flag | Matches | Self / external | Nat bits | Encoded bytes | Index result |
| --- | ---: | --- | ---: | ---: | --- |
| false | 1 | 1 / 0 | 257 | 131 | Executed, authenticated caller |
| true | 0 | 0 / 0 | — | 65 | Executed, authenticated caller |
| true | 1 | 1 / 0 | 257 | 131 | Executed, authenticated caller |
| true | 64 | 32 / 32 | 32,769 | 302,177 | Executed, authenticated caller |
| true | 256 | 128 / 128 | 4,097 | 159,938 | Executed, authenticated caller |

No default skipping-quota rejection occurred. Inspection of the exact pinned
[ic-cdk-macros 0.20.1 source](https://docs.rs/crate/ic-cdk-macros/0.20.1/source/src/export.rs)
explains this: the empty argument-tuple branch emits no argument decoder.
It never reads/traverses the record. Argument-bearing methods instantiate a
Candid decoder with a 10,000 skipping quota; that is not the zero-argument
method's execution path. No custom decoder or quota override is used.
[Candid's subtyping rules](https://github.com/dfinity/candid/blob/master/spec/Candid.md#upgrading-and-subtyping)
permit ignoring surplus arguments; native Candid skip probes alone would not
characterize this CDK-generated Wasm entry point.

The unchanged full-record receiver regression also passes 256 distinct
29-byte subjects, full-width targets/block indices and 257-, 4,097- and
32,769-bit amounts, with both global flag values. Its largest tested message
is 1,210,946 bytes. This guarantee was retained and not replaced by Index tests.

## Compatibility conclusion and Index prerequisite

The proposed zero-argument method is compatible with all measured envelopes
on this pinned Rust CDK. This is not a promise of unlimited size or behavior
in another CDK/language or the future Index's actual binary. IC message-size,
execution and cycle limits still apply. A zero-argument receiver cannot inspect
or filter the global flag, subjects, targets, thresholds, indices or amounts;
even specific-only callbacks become wakes.

Recommend the full `poke(PokeNotification)` method whenever complete legitimate
payload coverage or local filtering is required. Before production integration,
the Index project must either adopt that method or validate its actual
argument-discarding implementation against the full intended envelope range.
It must authenticate Event Horizon, bound/coalesce work, use its authoritative
Ledger cursor and retain periodic polling. Receipt never proves Index catch-up.
There is no Event Horizon Index dependency, capability branch, fallback, retry,
payload truncation or special-case suppression of specific matches.

## Documentation and validation

The subscriber guide, backend README, architecture overview, cadence guide and
read-only production query comment now describe the current record and three
queries. Historical reports retain their evidence with explicit supersession
notes. The normative `SPEC.md` and `candid/subscriber.did` are unchanged.

Focused tests passed; full release-gate results, inventories and checked artifact
identities will be recorded here after final validation.
