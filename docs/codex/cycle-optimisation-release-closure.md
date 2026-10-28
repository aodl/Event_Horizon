# Cycle optimisation release closure — progress

Starting source: `9528da787f9e8a4d5df2d297401564d04adb41b6`.
The published branch was independently confirmed by `git ls-remote`; main remains
`81c5a0d72ff188f5ee9f412deb20566a12307642`. Earlier publication failures describe
historical session permissions, not current branch visibility.

No production backend source or security policy changed. Existing 412-test,
72-pair, stable-state/upgrade and two-build evidence remains preserved.

Security: authoritative data is still overbroad; independent source inspection and
191 native panic cases support the maintainer's narrower range. See the
[ready correction request](block-buffer-advisory-correction-request.md). Submission
and final scan remain pending; no exception is authorised or activated.

Compiler selection: backend 2 / frontend full workspace z. The selected recipe
builds the same two-package set twice, preserving dependency feature unification;
the frontend output uses a separate target directory. The additional backend-z
output is discarded. Package-only z leaves dependencies at2 and is rejected for
its much smaller benefit. Frontend-only z also changes Cargo feature unification;
its measurements remain as a separate control, not the selected recipe.

| Frontend option | Bytes | Install/certification cycles | Reported idle/day | Heap |
|---|---:|---:|---:|---:|
| Current 2 | 4,009,935 | 13,914,479,233 | 1,176,063,315 | 3,997,696 |
| Package-only z | 3,975,247 | 13,687,637,098 | 1,175,177,104 | 3,997,696 |
| Standalone z control | 3,757,926 | 12,460,781,460 | 1,169,624,979 | 3,997,696 |
| Selected workspace z | 3,757,901 | 12,460,593,460 | 1,169,624,340 | 3,997,696 |

All four options pass twenty certified-response checks against identical current
embedded assets. Encoded response lengths can differ with certificate proofs.
Query statistics report13/20known calls; no exact query saving is inferred.
Installation savings are one-time; idle estimates are separate from query work.
The package override build reproduces backend f93cd920…8970 exactly. Final
canonical verification of the new recipe remains pending; local selected frontend
hash210d4de6…41c8 is provisional, not release evidence. Backend pin is unchanged.

Resume: observations/release-closure/run-validation.py (status.json and per-command
logs). Full gates and canonical two-build/population must finish before engineering
closure. PR/CI and upstream advisory correction remain pending. No policy exception
or production Rust/Candid/stable-layout change has been made. No deployment.
