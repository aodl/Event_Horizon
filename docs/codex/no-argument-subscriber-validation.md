# Zero-argument subscriber validation

Date: 2026-10-10

Starting revision: `1020b4c7008d9329e4be722c033d3c863b0e3def`

## Contract and implementation

Event Horizon retains one callback wire format: the production sender calls
`poke` with one `Vec<PokeMatch>` argument and submits that call one-way. No
signature setting, capability discovery, fallback call, acknowledgement, or
retry was added. The only production-source edit is a comment explaining why
the same payload-bearing call is compatible with a receiver that declares
`poke : () -> ()`.

The compatibility basis is Candid's whole-argument-sequence coercion: a callee
that expects fewer arguments ignores extras. The primary references inspected
were the [Candid
specification](https://github.com/dfinity/candid/blob/master/spec/Candid.md)
at `dfinity/candid` revision
`10423fae0c84f6a3f13abf1042f28f2ae40ac3af`, the [ICP Candid calling
guide](https://docs.internetcomputer.org/guides/canister-calls/candid/), and
the [ic-cdk `Call`
documentation](https://docs.rs/ic-cdk/0.20.1/ic_cdk/call/struct.Call.html).
The locked dependencies exercised here are `candid 0.10.29`, `ic-cdk 0.20.1`,
and `pocket-ic 15.0.0`.

## Fixture and coverage

`mock-subscriber-no-args` uses an ordinary CDK-generated update entry point:

```candid
poke : () -> ();
```

Its only observations are `debug_pokes : () -> (nat64) query` and
`debug_last_caller : () -> (opt principal) query`. The ordinary unit test
`generated_candid_has_zero_argument_update_poke` reads the interface generated
from the Rust entry points and requires exactly one zero-argument, zero-result,
non-query `poke` declaration. The existing vector fixture remains in use and
now also records its caller.

| Requirement | Evidence |
| --- | --- |
| Empty argument sequence, one `null`, and one empty vector are distinct | `clients::subscriber::tests::candid_poke_argument_compatibility_matrix` |
| Full decoder compatibility matrix and exact representative payload round trip | `clients::subscriber::tests::candid_poke_argument_compatibility_matrix` |
| Both target variants and `Nat > u64::MAX` | unit matrix and direct PocketIC entry-point test |
| Existing 256-match callback bound remains compatible | `clients::subscriber::tests::zero_argument_receiver_accepts_maximum_callback_payload` |
| Real generated entry points receive identical empty/nonempty vector bytes | `tests::generated_subscriber_entrypoints_obey_candid_argument_compatibility` |
| True zero arguments execute the zero-argument receiver and reject the vector receiver | `tests::generated_subscriber_entrypoints_obey_candid_argument_compatibility` |
| Shared legacy ICP global and specific production-path callbacks | `tests::shared_icp_mixed_receiver_styles_cover_global_specific_and_coalescing` |
| Two receiver styles coexist without negotiation or fallback | `tests::shared_icp_mixed_receiver_styles_cover_global_specific_and_coalescing` |
| Specific precedence, multi-transfer coalescing, and no-new-work behavior | `tests::shared_icp_mixed_receiver_styles_cover_global_specific_and_coalescing` |
| Separate ICRC-3 observation with canonical ICP-funded admission | `tests::generic_icrc3_specific_activity_executes_both_receiver_styles` |
| Receiver execution and caller identity, rather than one-way submission alone | all three PocketIC cases use fresh receiver counts; production-path cases require the Event Horizon principal as caller |

The arity-sensitive PocketIC test supplies `candid::encode_args(())` directly.
The general repository `update` and `query` helpers deliberately remain
unchanged because passing Rust `()` to them invokes `encode_one(())`, producing
one Candid `null` argument rather than an empty argument sequence.

## Validation results

Targeted results before the complete gates:

- production subscriber compatibility unit tests: 2 passed;
- generated no-argument fixture interface test: 1 passed;
- direct generated-entry-point PocketIC test: 1 passed;
- shared ICP mixed-receiver PocketIC test: 1 passed;
- separate ICRC-3 mixed-receiver PocketIC test: 1 passed.

The complete managed command
`POCKET_IC_MUTE_SERVER=1 cargo run --locked -p xtask -- test_all` passed 109
suites and 215 tests: 87 ordinary Rust unit/runner tests, 24 frontend tests,
and 104 separately executed PocketIC scenarios. Ignored-test discovery listed
all 104 PocketIC cases before execution. Formatting passed, locked workspace
Clippy passed with warnings denied, and the security gate passed with its four
existing reviewed/allowed warnings and no unfiltered advisory or vulnerability.

The three new integration scenarios can be rerun individually with:

```sh
POCKET_IC_MUTE_SERVER=1 cargo test --locked -p event-horizon-pocketic --lib -- --ignored --exact tests::generated_subscriber_entrypoints_obey_candid_argument_compatibility
POCKET_IC_MUTE_SERVER=1 cargo test --locked -p event-horizon-pocketic --lib -- --ignored --exact tests::shared_icp_mixed_receiver_styles_cover_global_specific_and_coalescing
POCKET_IC_MUTE_SERVER=1 cargo test --locked -p event-horizon-pocketic --lib -- --ignored --exact tests::generic_icrc3_specific_activity_executes_both_receiver_styles
```

## Canonical artifacts

The implementation was committed cleanly at artifact-source revision
`a01ac6b9b77e47b8048f5cd027a93bc27f6f65d6`. Two independent no-cache builds
from that revision were byte-identical. The subsequent evidence revision
changes only this report and the generated source manifest.

| Measured item | SHA-256 |
| --- | --- |
| `event_horizon.wasm` | `9775fa94617e4c089e9b7420129de0392e870d1d8ea4281f426bd1890c72721a` |
| `event_horizon_frontend.wasm` | `4f2a5b873f4ebef34d15cfc60f6185c9e40ab9dc70af4f3de0ddde1f3a871cde` |
| `build-info.json` | `d40c68e141665c8fd73ad098fe0fed7623f868123c8094896dc0f242f2a1f037` |
| artifact checksum manifest | `eb39713fd273fdb2c2e7c97428a5a9a19d34859d9d3836e5931006a18a16cb00` |
| artifact-source `MANIFEST.sha256` | `0d25a466e0ee8144bfccc0ec5b4963e68c4301c03f0ac44fdd9e970ef843cb03` |

`npm run verify:reproducible-artifacts`, `./tools/scripts/docker-build`, and
the subsequent `sha256sum -c release-artifacts.sha256` verification all
passed. The backend and frontend WebAssembly export audits passed with their
canonical 10 and 7 exports respectively; no fixture/debug method was present
in either production artifact. The production backend Candid file and frontend
Candid file are unchanged from the starting revision.

## Limitations

This establishes Candid decoding and observed handler execution in the locked
unit and PocketIC environments. It does not change the existing best-effort
one-way delivery contract into a delivery guarantee. A zero-argument receiver
discards targets and maximum amounts and therefore must use the notification
only to accelerate its own authenticated, authoritative reconciliation.
