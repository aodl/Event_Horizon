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

Observed Index receiver balance deltas across these cases range from 5,020,996
to 5,021,546 cycles. These measurements apply to this PocketIC fixture.

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

The subscriber guide, backend README, architecture overview, cadence guide,
measurement notes, test assertion messages and read-only production query
comment now describe independent record fields and the three queries.
Direct-Ledger reconciliation, Index catch-up, local thresholds and multi-ledger
examples use the same contract. Historical reports retain their evidence with
explicit supersession notes. The normative `SPEC.md` and
`candid/subscriber.did` are unchanged.

Implementation commit: `60d1a1a796bbba380961bb5f3b8cec519a1539be`.
Additional test/measurement wording cleanup:
`99b15f4dd109ad38d22155ad83900880d617f0c2`.
The final report/manifest commit adds release evidence only; its exact revision
is identified in the completion report and canonical build logs.

| Check | Result |
| --- | --- |
| Fixture exported-Candid unit test | Passed; genuine zero-argument update |
| Focused PocketIC | All three Index scenarios and unchanged maximum full-record regression passed |
| `cargo fmt --all -- --check` | Passed, including after final wording cleanup |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed, including after final wording cleanup |
| `cargo run -p xtask -- test_unit` | Passed: 134 Rust + 138 frontend tests; static, manifest and canonical-context gates passed |
| `POCKET_IC_MUTE_SERVER=1 cargo run -p xtask -- test_all` | Full rerun passed: 126 suites, 393 tests, including all 121 isolated PocketIC scenarios |
| Wording-touched PocketIC regressions | Both passed again, including the renamed global/mixed notification test |
| `./tools/scripts/security-scan` | Passed with unchanged existing advisory exceptions; no new exceptions or runtime dependencies |
| `npm run verify:reproducible-artifacts` | Passed; complete release artifacts byte-identical across two independent clean Docker builds |

The first full run completed 120 PocketIC scenarios successfully, but
`external_shared_account_fanout_1024_and_many_transfers` lost a local PocketIC
HTTP response with `hyper::Error(IncompleteMessage)` while awaiting ingress.
Its log is retained. After concurrent builds finished, a complete rerun passed
all 121 scenarios, including every ICP/generic 1,024-watcher capacity case.
No callback logic, notification limits, decoding quotas or production retry
behavior changed to obtain that result. The final wording-only cleanup was
also checked with formatting, Clippy, all units and both affected scenarios.

## Canonical artifacts, exports and upgrade evidence

Canonical builds use clean archived source contexts. The final completion
report records the exact clean evidence revision used for final builds.
The checked production artifacts retain the reviewed baseline identities:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Backend | 1,428,428 | `2792706c1100d23da6f9980cf3412d544edf87417c1ea9dad12ea3b661248528` |
| Frontend | 3,757,773 | `338565e50d946d09201be87131d9c7f5319923320bf68926a17a4f6d19582067` |

The release artifact manifest verifies, including `build-info.json`; its input
hashes match the source. Adding the test-only workspace package updates
`Cargo.lock` and therefore build-info metadata, without changing either Wasm.
The frontend's reviewed backend pin is unchanged and verified in its source,
bundle and canonical Wasm.

Export audits pass for the backend (11 total exports; application methods only
the three approved read-only queries) and frontend (7 total exports;
`http_request` is its only application method). The production-Wasm PocketIC
surface regression also passes. Production Wasm exports no Index fixture or
observation methods.

The actual old-Wasm upgrade regression
`pre_alias_layout_upgrades_without_rewriting_financial_or_subscription_state`
passes, as do the retained current-schema upgrade tests. The historical debug
fixture's SHA-256 remains
`093a43a8cb52d560829654819b5c4bad42cc955b7c34bafa3f734c416f4643b0`.
Stable-memory layout, subscription semantics, funding state machine and the
production notification sender are unchanged.

Raw local logs, including the initial HTTP interruption and successful full
rerun, are preserved under the ignored `observations/index-poke/` directory.
The remaining Index deployment prerequisite is validation of its actual
receiver binary or adoption of the full record, together with authentication,
coalescing, authoritative cursors and periodic polling described above.
