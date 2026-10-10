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

Canonical artifact hashes and the artifact-source revision are added after the
clean-commit reproducibility builds so that those values describe measured
outputs rather than a dirty working tree.

## Limitations

This establishes Candid decoding and observed handler execution in the locked
unit and PocketIC environments. It does not change the existing best-effort
one-way delivery contract into a delivery guarantee. A zero-argument receiver
discards targets and maximum amounts and therefore must use the notification
only to accelerate its own authenticated, authoritative reconciliation.
