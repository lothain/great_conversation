// Who is calling: verify the Cloudflare Access token on a request.
//
// Access sits in front of /auth/* and adds a signed JWT (`Cf-Access-Jwt-Assertion`) to every
// request it lets through. We verify it ourselves (signature, audience, issuer, expiry) rather
// than trusting a header, so nothing reaching the Function directly can pretend to be anyone.
//
// Environment (wrangler.toml [vars] / dashboard):
//   ACCESS_TEAM_DOMAIN  https://<team>.cloudflareaccess.com
//   ACCESS_AUD          the Access application's "Application Audience (AUD) Tag"
//   CURATOR_EMAILS      comma-separated emails allowed to edit the curator's text
//   DEV_EMAIL           local development only (`wrangler pages dev`, .dev.vars): act as this
//                       email without Access. Never set it in production.

let certs = { at: 0, keys: new Map() };

async function keys(team) {
  if (Date.now() - certs.at < 10 * 60 * 1000) return certs.keys;
  const r = await fetch(`${team}/cdn-cgi/access/certs`);
  if (!r.ok) throw new Error(`access certs: ${r.status}`);
  const { keys: jwks } = await r.json();
  const map = new Map();
  for (const jwk of jwks) {
    const key = await crypto.subtle.importKey("jwk", jwk, { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" }, false, ["verify"]);
    map.set(jwk.kid, key);
  }
  certs = { at: Date.now(), keys: map };
  return map;
}

const b64url = s => Uint8Array.from(atob(s.replace(/-/g, "+").replace(/_/g, "/") + "===".slice((s.length + 3) % 4)), c => c.charCodeAt(0));
const decode = s => JSON.parse(new TextDecoder().decode(b64url(s)));

/// The verified email of the caller, or null.
export async function callerEmail(request, env) {
  return (await caller(request, env)).email;
}

/// { email } or { email: null, why }: why the token was refused (no secrets; safe to return).
export async function caller(request, env) {
  if (env.DEV_EMAIL) return { email: env.DEV_EMAIL.toLowerCase() };
  const no = why => ({ email: null, why });
  if (!env.ACCESS_TEAM_DOMAIN || !env.ACCESS_AUD) return no("server not configured");
  const token = request.headers.get("Cf-Access-Jwt-Assertion");
  if (!token) return no("no Access token on the request");
  const [h, p, sig] = token.split(".");
  if (!h || !p || !sig) return no("malformed token");
  try {
    const header = decode(h), payload = decode(p);
    const key = (await keys(env.ACCESS_TEAM_DOMAIN)).get(header.kid);
    if (!key) return no(`unknown signing key ${header.kid}`);
    if (!(await crypto.subtle.verify("RSASSA-PKCS1-v1_5", key, b64url(sig), new TextEncoder().encode(`${h}.${p}`)))) return no("bad signature");
    const aud = Array.isArray(payload.aud) ? payload.aud : [payload.aud];
    if (!aud.includes(env.ACCESS_AUD)) return no(`audience ${aud.join(",")} is not this app`);
    if (payload.iss !== env.ACCESS_TEAM_DOMAIN) return no(`issuer ${payload.iss} is not ${env.ACCESS_TEAM_DOMAIN}`);
    if (!(payload.exp > Math.floor(Date.now() / 1000))) return no("token expired");
    if (typeof payload.email !== "string") return no("token has no email");
    return { email: payload.email.toLowerCase() };
  } catch (e) {
    return no(`verify failed: ${e.message}`);
  }
}

export function isCurator(email, env) {
  const allowed = (env.CURATOR_EMAILS || "").split(",").map(s => s.trim().toLowerCase()).filter(Boolean);
  return !!email && allowed.includes(email);
}

export const json = (body, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json; charset=utf-8", "cache-control": "no-store" } });
