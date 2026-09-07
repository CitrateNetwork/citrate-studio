#!/usr/bin/env bash
# ST-B-019 tripwire — RunState.approved has exactly ONE production writer.
#
# The High-risk approval gate advances only through `apply_resume`, which
# fail-closes on `quorum_met()`. Any other `approved.push(...)` in production
# code is an ungated bypass (the audited `CITRATE_STUDIO_SEED=done` handler
# wrote `approved` directly, skipping the quorum check). This check fails CI on
# any `approved.push` outside `fn apply_resume`, and on any non-test reference
# to the dev-only screenshot harness (`headless_shot` / `apply_seed`) that is
# not gated behind `#[cfg(debug_assertions)]`.
#
# Runs from the repo root. Exit 0 = clean, 1 = violation.
set -euo pipefail

cd "$(dirname "$0")/../.."
SRC="src/main.rs"
fail=0

# --- 1. Single-writer invariant: approved.push only inside apply_resume -------
# Strip #[cfg(test)] modules are hard to bound in bash, so we allow-list the
# known test writer explicitly by line content and flag everything else.
mapfile -t hits < <(grep -nE '\.approved\.push\(' "$SRC" || true)
for h in "${hits[@]}"; do
    line="${h%%:*}"
    text="${h#*:}"
    # The guarded production writer.
    if grep -qE '^\s*s\.approved\.push\(id\);' <<<"$text"; then
        # Confirm it sits inside apply_resume by scanning the nearest fn above.
        fn=$(awk -v n="$line" 'NR<=n && /^\s*fn /{f=$0} END{print f}' "$SRC")
        if grep -q 'fn apply_resume' <<<"$fn"; then
            continue
        fi
    fi
    # A test-module writer is acceptable (skips the real gate deliberately).
    fn=$(awk -v n="$line" 'NR<=n && /#\[test\]/{t=1} NR<=n && /^\s*fn /{f=$0} END{print (t?"TEST ":"") f}' "$SRC")
    if grep -q '^TEST ' <<<"$fn"; then
        continue
    fi
    echo "ST-B-019 VIOLATION: production approved.push outside apply_resume at $SRC:$line"
    echo "    $text"
    fail=1
done

# --- 2. The dev-only harness must be cfg(debug_assertions)-gated -------------
for fn in headless_shot apply_seed; do
    defline=$(grep -nE "^\s*fn ${fn}\b" "$SRC" | head -1 | cut -d: -f1 || true)
    if [[ -z "$defline" ]]; then
        continue
    fi
    prev=$((defline - 1))
    # Walk back over doc comments to the attribute directly above the fn.
    while [[ $prev -gt 1 ]]; do
        t=$(sed -n "${prev}p" "$SRC")
        if grep -qE '^\s*///' <<<"$t"; then prev=$((prev - 1)); continue; fi
        break
    done
    attr=$(sed -n "${prev}p" "$SRC")
    if ! grep -q 'cfg(debug_assertions)' <<<"$attr"; then
        echo "ST-B-019 VIOLATION: fn ${fn} at $SRC:$defline is not #[cfg(debug_assertions)]-gated"
        echo "    (a release binary must not carry the env-var screenshot harness)"
        fail=1
    fi
done

if [[ $fail -eq 0 ]]; then
    echo "ST-B-019 OK: approved has one guarded writer; dev harness is debug-only."
fi
exit $fail
