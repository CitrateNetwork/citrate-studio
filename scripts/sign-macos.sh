#!/usr/bin/env bash
# Sign (+ verify, + notarize-if-creds) the macOS .app. Real Developer-ID signing
# when the cert is in the keychain; the notarization step activates when a notary
# keychain profile exists (xcrun notarytool store-credentials <profile>).
#
#   scripts/sign-macos.sh [path/to/App.app]
#
# Env: SIGN_IDENTITY (default: the Developer ID Application in the keychain)
#      NOTARY_PROFILE (a stored notarytool keychain profile; skipped if unset)
set -euo pipefail
cd "$(dirname "$0")/.."

APP="${1:-target/release/bundle/osx/Citrate Studio.app}"
IDENTITY="${SIGN_IDENTITY:-Developer ID Application}"
ENTITLEMENTS="packaging/entitlements.plist"

[ -d "$APP" ] || { echo "no app at: $APP (run: cargo bundle --release)"; exit 1; }

echo "▸ codesign (hardened runtime, timestamped) — $IDENTITY"
codesign --force --deep --options runtime --timestamp \
  --entitlements "$ENTITLEMENTS" \
  --sign "$IDENTITY" "$APP"

echo "▸ verify"
codesign --verify --deep --strict --verbose=2 "$APP"
echo "▸ Gatekeeper assessment (pre-notarization will report 'rejected' until notarized)"
spctl --assess --type execute --verbose "$APP" || true

if [ -n "${NOTARY_PROFILE:-}" ]; then
  echo "▸ notarize via profile '$NOTARY_PROFILE'"
  DITTO_ZIP="$(mktemp -d)/CitrateStudio.zip"
  ditto -c -k --keepParent "$APP" "$DITTO_ZIP"
  xcrun notarytool submit "$DITTO_ZIP" --keychain-profile "$NOTARY_PROFILE" --wait
  xcrun stapler staple "$APP"
  echo "▸ notarized + stapled"
else
  echo "▸ NOTARY_PROFILE unset — signed but not notarized."
  echo "  To notarize: xcrun notarytool store-credentials <profile> --apple-id … --team-id DDHUG44QC7 --password <app-specific>"
  echo "  then: NOTARY_PROFILE=<profile> scripts/sign-macos.sh"
fi
