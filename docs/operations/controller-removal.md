# Controller removal

Event Horizon uses **an empty controller list** for final immutability. It does not transfer control to a blackhole canister.

This procedure is intentionally manual and irreversible. There is no in-protocol administration or self-upgrade mechanism.

## Preconditions

Do not continue unless all of the following are true:

- the controlled mainnet observation period has completed satisfactorily;
- canonical reproducible-build verification passes;
- the installed backend module hash matches `release-artifacts/event_horizon.wasm`;
- the production Wasm export audit passes;
- `status_visibility` is `public`;
- `log_visibility` is `public`;
- `log_memory_limit` is exactly 16384 bytes;
- the canister has a healthy cycles reserve;
- `get_instance` and recent public `CONFIG`/`HEALTH` logs report the intended immutable observed Ledger, SNS Root, and surplus recipient;
- the fixed Protocol ICP Ledger, CMC, Historian and Faucet constants are correct;
- install-time `surplus_canister` is either deliberately `None` or the final reviewed immutable receiver principal; no destination decision remains open;
- if surplus is enabled, retained-first ordering, epoch policy, immediate gates, duplicate recovery, and pending-state upgrades have passed against the canonical Wasm;
- the reviewed Historian D/E bindings and E. payment route are installed; E points to this backend;
- the state-preserving backend upgrade and coordinated receiver/frontend upgrades have completed and their installed hashes match review;
- pricing freezes/activation and `get_pricing` agree with the frontend during the separate observation period;
- recurring Faucet receipts and successful retained CMC conversions have been
  observed, the cycles reserve is adequate for the intended cadence, and the
  sustainable operating plan is understood from measured receipts, polling,
  pokes, and burn rather than hypothetical yield alone;
- the planned 3,000 ICP productive protocol endowment, subscriber endowments,
  and liquid-cycles bootstrap have been tracked as distinct funding categories;
- the Wasm export audit permits exactly `canister_query get_instance`, `canister_query get_pricing`, and `canister_query preview_subscription`, with no update/admin application method;
- the frontend Wasm export audit permits exactly `canister_query http_request` and rejects `http_request_update`;
- ordinary single/range, neuron single/range, and global Historian admission paths have been tested against the current prices;
- backend and frontend artifacts reproduce byte-for-byte and the frontend's
  reviewed static backend hash equals the installed backend module hash;
- no remaining operational task requires controller access.

## Inspect

```bash
icp canister status event_horizon -e ic
icp canister settings show event_horizon -e ic
sha256sum release-artifacts/event_horizon.wasm
```

Record the canister ID, module hash, current controllers, cycle balance, settings, and complete immutable `get_instance` tuple in the release record.

## Make immutable

The current `icp` CLI supports replacing the controller list with an empty list:

```bash
icp canister settings update event_horizon -e ic --remove-all-controllers
```

**This is irreversible.** Do not run it as part of an automated deployment script.

## Verify afterwards

Use public status/read-state information to verify:

- controllers are empty;
- module hash is unchanged;
- status visibility remains public;
- log visibility remains public;
- the canister remains running and funded.

A controllerless canister cannot have its code or settings changed by an external party. Event Horizon also contains no
self-upgrade/install-code path, which is checked statically and should remain part of every release review.

Current ICP guidance: https://docs.internetcomputer.org/guides/canister-management/settings/
