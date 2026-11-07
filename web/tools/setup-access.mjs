// Set up Cloudflare Access (email one-time PIN) in front of /auth/* — no dashboard clicking.
//
//   $env:CLOUDFLARE_API_TOKEN = "..."     # token with the permissions below
//   $env:CLOUDFLARE_ACCOUNT_ID = "..."
//   node tools/setup-access.mjs great-conversation.pages.dev [more.hostnames ...]
//
// Token permissions (My Profile → API Tokens → Create Custom Token), all "Account" scope:
//   Access: Apps and Policies → Edit
//   Access: Organizations, Identity Providers, and Groups → Edit
//
// Idempotent: reuses the one-time PIN login method and the application if they exist.
// Writes ACCESS_TEAM_DOMAIN and ACCESS_AUD into wrangler.toml; then redeploy.

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const APP_NAME = "Great Conversation sign-in";
const token = process.env.CLOUDFLARE_API_TOKEN;
const account = process.env.CLOUDFLARE_ACCOUNT_ID;
const hosts = process.argv.slice(2);
if (!token || !account || !hosts.length) {
  console.error("usage: set CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID, then\n  node tools/setup-access.mjs <hostname> [more hostnames]");
  process.exit(2);
}

async function cf(method, path, body) {
  const r = await fetch(`https://api.cloudflare.com/client/v4/accounts/${account}${path}`, {
    method,
    headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await r.json().catch(() => ({}));
  if (!r.ok || data.success === false) {
    const why = (data.errors || []).map(e => `${e.code}: ${e.message}`).join("; ") || r.statusText;
    throw new Error(`${method} ${path} → ${r.status} ${why}`);
  }
  return data.result;
}

// 1. The Zero Trust organization gives the team domain.
let org;
try {
  org = await cf("GET", "/access/organizations");
} catch (e) {
  console.error("No Zero Trust organization found. Open https://one.dash.cloudflare.com once to pick a team name (free plan), then rerun.\n", e.message);
  process.exit(1);
}
const teamDomain = `https://${org.auth_domain}`;
console.log("team domain:", teamDomain);

// 2. One-time PIN login method.
const idps = await cf("GET", "/access/identity_providers");
let otp = idps.find(p => p.type === "onetimepin");
if (!otp) {
  otp = await cf("POST", "/access/identity_providers", { name: "One-time PIN", type: "onetimepin", config: {} });
  console.log("enabled one-time PIN login");
} else {
  console.log("one-time PIN login already enabled");
}

// 3. The application: /auth/* on each hostname, PIN login only, anyone with an email may sign in
//    (the site's own code decides who may edit).
const destinations = hosts.map(h => ({ type: "public", uri: `${h}/auth/*` }));
const apps = await cf("GET", "/access/apps");
let app = apps.find(a => a.name === APP_NAME);
if (!app) {
  app = await cf("POST", "/access/apps", {
    name: APP_NAME,
    type: "self_hosted",
    domain: `${hosts[0]}/auth/*`,
    destinations,
    allowed_idps: [otp.id],
    auto_redirect_to_identity: true,
    session_duration: "720h",
    app_launcher_visible: false,
  });
  await cf("POST", `/access/apps/${app.id}/policies`, {
    name: "Anyone with an email",
    decision: "allow",
    include: [{ everyone: {} }],
    precedence: 1,
  });
  console.log("created application", app.id);
} else {
  console.log("application exists:", app.id);
}
console.log("AUD tag:", app.aud);

// 4. Write both into wrangler.toml.
const toml = join(dirname(fileURLToPath(import.meta.url)), "..", "wrangler.toml");
const text = readFileSync(toml, "utf8")
  .replace(/^ACCESS_TEAM_DOMAIN = ".*"$/m, `ACCESS_TEAM_DOMAIN = "${teamDomain}"`)
  .replace(/^ACCESS_AUD = ".*"$/m, `ACCESS_AUD = "${app.aud}"`);
writeFileSync(toml, text);
console.log("updated", toml, "— now redeploy: npx wrangler pages deploy ../dist-release --project-name great-conversation");
