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
- `log_memory_limit` is at least 4096 bytes (the intended value is exactly 4096);
- the canister has a healthy cycles reserve;
- the configured Ledger, CMC, Historian and Faucet constants are correct;
- the Jupiter Faucet `X` alias points to this backend canister;
- pricing has initialized, at least one monthly freeze/activation has been observed, and `get_pricing` agrees with the frontend;
- the Wasm export audit permits exactly `canister_query get_pricing` and no other application method;
- account and global Historian admission totals have been tested against the current prices;
- no remaining operational task requires controller access.

## Inspect

```bash
icp canister status event_horizon -e ic
icp canister settings show event_horizon -e ic
sha256sum release-artifacts/event_horizon.wasm
```

Record the canister ID, module hash, current controllers, cycle balance and settings in the release record.

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
