# Agent instructions

Event Horizon deliberately optimises for inherent complexity only.

Before changing ICP-specific code, consult the current DFINITY ICP Skills guidance appropriate to the task (stable memory, multi-canister calls, Ledger, cycles management, canister security, testing, and certified frontend serving). Prefer current primary ICP documentation to remembered API details.

The root `SPEC.md` is normative. Do not add delivery retries, archive traversal, subscriber accounting, administrative APIs, arbitrary predicates, Index dependencies, dynamic polling economics, or generic framework layers unless the specification is explicitly amended first.

Production and debug/test surfaces must remain separate. Test-only configuration or methods must never be exported by the canonical production Wasm.
