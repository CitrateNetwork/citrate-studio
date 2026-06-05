#!/usr/bin/env bash
#
# Visual e2e harness — captures every Citrate Studio surface/action as a PNG via
# the headless software renderer, builds an HTML gallery, and (in `check` mode)
# diffs against committed golden baselines.
#
# Citrate Studio is a NATIVE Slint app, so this is a software-renderer snapshot
# harness, not Playwright (which only drives web browsers — see README in
# tests/web-playwright/). Each state is reached through the app's real env seeds.
#
# Usage:
#   scripts/visual-harness.sh capture   # render all states → gallery
#   scripts/visual-harness.sh update    # render all states → refresh baselines
#   scripts/visual-harness.sh check     # render all states → diff vs baselines (CI gate)
#
# Env:
#   STUDIO_CORE_LIVE=1   build/run with --features core-live (real dispatch/doctor/chain)

set -euo pipefail
cd "$(dirname "$0")/.."

MODE="${1:-capture}"
OUT="docs/visual/gallery"
BASE="docs/visual/baseline"
WALLET="0x1a2b3c4d5e6f7890abcdef1234567890de"

FEATURES=""
RUNENV=""
if [[ "${STUDIO_CORE_LIVE:-}" == "1" ]]; then
  FEATURES="--features core-live"
  RUNENV="CITRATE_CAPSULES_DIR=../citrate-agent-runtime/capsules"
fi

echo "▸ building (${FEATURES:-default})…"
cargo build $FEATURES >/dev/null 2>&1
BIN="target/debug/citrate-studio"

mkdir -p "$OUT"
rm -f "$OUT"/*.png

# name|env-vars   (env-vars are space-separated KEY=VALUE; CITRATE_STUDIO_SHOT is added)
SPECS=(
  "01-studio-idle|"
  "02-run-running|CITRATE_STUDIO_SEED=running"
  "03-gate-dock|CITRATE_STUDIO_SEED=gate"
  "04-run-done|CITRATE_STUDIO_SEED=done"
  "05-signed-in|CITRATE_STUDIO_SIGNEDIN=$WALLET CITRATE_STUDIO_KYC=verified"
  "06-inspector|CITRATE_STUDIO_SELECT=c3"
  "07-code-drawer|CITRATE_STUDIO_OVERLAY=code"
  "08-health-report|CITRATE_STUDIO_OVERLAY=health"
  "09-break-glass|CITRATE_STUDIO_OVERLAY=breakglass"
  "10-agent-chat|CITRATE_STUDIO_OVERLAY=chat"
  "11-settings-rbac|CITRATE_STUDIO_OVERLAY=settings CITRATE_STUDIO_SECTION=rbac"
  "12-settings-roster|CITRATE_STUDIO_OVERLAY=settings CITRATE_STUDIO_SECTION=roster"
  "13-settings-grants|CITRATE_STUDIO_OVERLAY=settings CITRATE_STUDIO_SECTION=grants"
  "14-settings-runtime|CITRATE_STUDIO_OVERLAY=settings CITRATE_STUDIO_SECTION=runtime"
  "15-settings-policy|CITRATE_STUDIO_OVERLAY=settings CITRATE_STUDIO_SECTION=policy CITRATE_STUDIO_CHAIN=1"
  "16-settings-account|CITRATE_STUDIO_OVERLAY=settings CITRATE_STUDIO_SECTION=account CITRATE_STUDIO_SIGNEDIN=$WALLET CITRATE_STUDIO_KYC=verified"
  "17-scrubber-tampered|CITRATE_STUDIO_TAMPER=1"
  "18-onboarding|CITRATE_STUDIO_VIEW=onboard CITRATE_STUDIO_WS=beginner"
  "19-beginner-chat|CITRATE_STUDIO_WS=beginner"
)

echo "▸ capturing ${#SPECS[@]} surfaces…"
for spec in "${SPECS[@]}"; do
  name="${spec%%|*}"
  vars="${spec#*|}"
  env $RUNENV $vars CITRATE_STUDIO_SHOT="$OUT/$name.png" "$BIN" >/dev/null 2>&1
  printf '  ✓ %s\n' "$name"
done

python3 scripts/gallery.py "$OUT" "${SPECS[@]}"
echo "▸ gallery → $OUT/index.html"

case "$MODE" in
  update)
    mkdir -p "$BASE"; cp "$OUT"/*.png "$BASE"/
    echo "▸ baselines refreshed → $BASE"
    ;;
  check)
    python3 scripts/visual-diff.py "$OUT" "$BASE"
    ;;
  capture) ;;
  *) echo "unknown mode: $MODE"; exit 2 ;;
esac
