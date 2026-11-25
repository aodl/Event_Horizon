# Event Horizon ICP worker

Build one Linux x86-64 executable for Webdock, netcup, and a Nibrun app with
manually enabled always-on activation. The trusted deployment computer builds
the artifact and prepares private configuration. The hosts run the worker
continuously afterward, even when that computer is off.

1. Build and validate the executable on the [trusted deployment computer](README.md#build-and-test).
2. Assign a distinct allowlisted signing identity and private checkpoint to each
   installation. The prepared pair's provider labels are interchangeable. Use
   the original start block `40615886` with those settings.
3. Follow [VPS setup](VPS_SETUP.md) or [Nibrun setup](NIBRUN_SETUP.md) and the
   later [hosted acceptance checklist](VALIDATION.md#hosted-acceptance).
4. Use the separate [customer subscription guide](SUBSCRIPTIONS.md) when you
   want Event Horizon to notify a receiver canister. A worker needs poke
   permission and can operate without a subscription, endowment, or ICP wallet
   balance.

The worker aims for one second between successful pass starts. Network time,
catch-up, and backend reconciliation all contribute to end-to-end delay.
Independent workers and interrupted checkpoint writes can repeat a wake.
