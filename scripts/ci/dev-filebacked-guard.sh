#!/usr/bin/env bash
# ST-B-011 tripwire — the `dev-filebacked` demo feature (which seeds a signer
# roster and writes its PRIVATE KEYS to disk in plaintext) must NEVER compile
# into a distributed release binary.
#
# The original guard keyed only on `debug_assertions`, which a profile override
# (`profile.release.debug-assertions = true`) flips back on inside an optimized
# build — re-opening the plaintext-key seed path. build.rs now also emits a
# `shipping_profile` cfg whenever PROFILE == "release", and signing.rs fires a
# second `compile_error!` on `all(feature = "dev-filebacked", shipping_profile)`.
#
# This check asserts the BYPASS still fails to compile. It is the negative test:
# a build that compiles here is a regression. Opt-in (a release check is slow).
#
# Runs from the repo root. Exit 0 = guard holds, 1 = guard bypassed.
set -uo pipefail
cd "$(dirname "$0")/../.."

echo "ST-B-011: asserting a release dev-filebacked build (debug-assertions forced on) does NOT compile…"
if cargo check --release --features dev-filebacked \
      --config 'profile.release.debug-assertions=true' >/tmp/st_b_011_check.log 2>&1; then
    echo "::error::dev-filebacked compiled into a release binary — the ST-B-011 guard is bypassed"
    exit 1
fi

if grep -q "audit ST-B-011\|shipping_profile\|audit F-3" /tmp/st_b_011_check.log; then
    echo "OK: the compile_error guard fired (dev-filebacked is blocked in release)."
    exit 0
fi

echo "::error::release dev-filebacked build failed, but NOT via the audit guard — investigate:"
tail -20 /tmp/st_b_011_check.log
exit 1
