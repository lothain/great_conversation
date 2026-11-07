// GET /auth/api/me: who is signed in.
// -> { id, email, displayName, tristan }   (id: the opaque author id that posts carry;
//    tristan: this is Tristan, who may edit Tristan's replies and contexts)
//    401 { error, why } when Access didn't vouch for anyone

import { caller, isTristan, json } from "../../_lib/access.js";
import { readerId } from "../../_lib/live.js";

export async function onRequestGet({ request, env }) {
  const { email, why } = await caller(request, env);
  if (!email) return json({ error: "not signed in", why }, 401);
  const row = await env.DB.prepare("SELECT display_name FROM users WHERE email = ?").bind(email).first();
  return json({ id: await readerId(email), email, displayName: row?.display_name ?? null, tristan: await isTristan(email, env) });
}
