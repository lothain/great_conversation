// PUT /auth/api/name  { displayName }   any signed-in reader; names are unique (case-insensitive).
// -> { displayName }   409 when someone else has it.

import { callerEmail, json } from "../../_lib/access.js";

export async function onRequestPut({ request, env }) {
  const email = await callerEmail(request, env);
  if (!email) return json({ error: "not signed in" }, 401);

  let body;
  try { body = await request.json(); } catch { return json({ error: "expected JSON" }, 400); }
  const name = typeof body?.displayName === "string" ? body.displayName.normalize("NFKC").trim().replace(/\s+/g, " ") : "";
  if (name.length < 2 || name.length > 40 || /[\p{C}<>]/u.test(name)) {
    return json({ error: "a display name is 2–40 characters, letters, spaces and punctuation" }, 400);
  }
  // Uniqueness ignores case: "Ada" and "ada" are the same name.
  const key = name.toLowerCase();
  const taken = await env.DB.prepare("SELECT email FROM users WHERE display_key = ? AND email <> ?").bind(key, email).first();
  if (taken) return json({ error: "that name is taken" }, 409);

  try {
    await env.DB.prepare(
      `INSERT INTO users (email, display_name, display_key, updated_at) VALUES (?, ?, ?, ?)
       ON CONFLICT (email) DO UPDATE SET display_name = excluded.display_name, display_key = excluded.display_key, updated_at = excluded.updated_at`,
    ).bind(email, name, key, new Date().toISOString()).run();
  } catch {
    return json({ error: "that name is taken" }, 409);   // lost a race to the UNIQUE index
  }
  return json({ displayName: name });
}
