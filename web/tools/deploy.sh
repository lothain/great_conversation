#!/usr/bin/env bash
# Build and deploy the Great Conversation to Cloudflare Pages, setting up sign-in on first run.
#
#   CF_ACCESS_TOKEN=... tools/deploy.sh [hostname]        (first run: sets up Cloudflare Access)
#   tools/deploy.sh [hostname]                            (later runs: build + deploy)
#
# Needs: `npx wrangler login` done once; Rust + trunk; Node.
# CF_ACCESS_TOKEN is an API token with Account → "Access: Apps and Policies → Edit" and
# "Access: Organizations, Identity Providers, and Groups → Edit". It's only used for the Access
# setup; wrangler keeps using your login (so the token never needs Pages/D1 rights).
#
set -euo pipefail
cd "$(dirname "$0")/.."                     # web/
HOST="${1:-great-conversation.pages.dev}"
PROJECT="great-conversation"
OUT="../dist-release"

# Keep any token in the environment away from wrangler, which would otherwise prefer it to the login.
wr() { env -u CLOUDFLARE_API_TOKEN npx --yes wrangler@4 "$@"; }

echo "== Cloudflare account"
ACCOUNT_ID="${CLOUDFLARE_ACCOUNT_ID:-$(wr whoami 2>/dev/null | grep -oE '[0-9a-f]{32}' | head -1 || true)}"
[ -n "$ACCOUNT_ID" ] || { echo "Couldn't find your account. Run: npx wrangler login"; exit 1; }
echo "account $ACCOUNT_ID"

if grep -q 'PASTE-THE-APPLICATION-AUD-TAG\|YOUR-TEAM' wrangler.toml; then
  echo "== Setting up sign-in (Cloudflare Access, email one-time PIN)"
  [ -n "${CF_ACCESS_TOKEN:-}" ] || { echo "First run needs CF_ACCESS_TOKEN (see the top of this script)."; exit 1; }
  CLOUDFLARE_API_TOKEN="$CF_ACCESS_TOKEN" CLOUDFLARE_ACCOUNT_ID="$ACCOUNT_ID" node tools/setup-access.mjs "$HOST"
else
  echo "== Sign-in already configured in wrangler.toml"
fi

echo "== Curator emails (a secret, kept out of the repo)"
if wr pages secret list --project-name "$PROJECT" 2>/dev/null | grep -q CURATOR_EMAILS; then
  echo "set"
else
  read -r -p "Emails allowed to edit the curator's text (comma-separated): " CURATORS
  [ -n "$CURATORS" ] || { echo "Needed once; rerun when ready."; exit 1; }
  printf '%s' "$CURATORS" | wr pages secret put CURATOR_EMAILS --project-name "$PROJECT"
fi

echo "== Database tables"
if wr d1 execute great-conversation --remote --json --command "SELECT name FROM sqlite_master WHERE type='table' AND name='posts'" 2>/dev/null | grep -q '"posts"'; then
  echo "up to date"
else
  echo "creating the missing tables"
  wr d1 execute great-conversation --remote --file=schema.sql >/dev/null
fi

echo "== Live room (WebSocket worker for new messages and typing)"
wr deploy -c live/wrangler.toml

echo "== Building (release, size-optimized)"
trunk build --release --cargo-profile wasm-release --dist "$OUT"

echo "== Deploying to production"
wr pages deploy "$OUT" --project-name "$PROJECT" --branch prod --commit-dirty=true

echo "== Checking https://$HOST"
sleep 5
code=$(curl -s -m 20 -o /dev/null -w '%{http_code}' "https://$HOST/")
edits=$(curl -s -m 20 "https://$HOST/api/edits" || true)
login=$(curl -s -m 20 -o /dev/null -w '%{redirect_url}' "https://$HOST/auth/login?return=/")
echo "page:    $code"
echo "edits:   $edits"
live=$(curl -s -m 20 -o /dev/null -w '%{http_code}' -H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' "https://$HOST/api/live")
echo "live:    $live (101 = working)"
case "$login" in
  *cloudflareaccess.com*) echo "sign-in: OK (goes to Cloudflare Access)";;
  *) echo "sign-in: NOT protected yet (login redirects to: $login). Access can take a minute; rerun the check, or see setup-access output.";;
esac
