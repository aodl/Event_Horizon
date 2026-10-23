# Historical receiver validation: superseded

The earlier receiver compatibility experiment is superseded by the deliberate
breaking unified `poke(PokeNotification)` contract. Compatibility-only receivers
and tests have been removed. The current contract is in
[subscriber.did](../../candid/subscriber.did); current receiving-canister,
upgrade, resource and release evidence is in the
[release report](permanent-aliases-subjects-release.md).

The subsequent [Index-specific interoperability follow-up](index-poke-validation.md)
measures a proposed zero-argument Index implementation against the unchanged
record-based sender. It does not restore the earlier general compatibility
arrangements or make that implementation the standard subscriber interface.
