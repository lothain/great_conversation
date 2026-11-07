// PUT /auth/api/edits  { id: "entry:<id>", field: "reply" | "context", text: string | null }
// Only the curator. `text: null` reverts to the original.

import { callerEmail, isCurator, json } from "../../_lib/access.js";

const MAX_TEXT = 20000;

export async function onRequestPut({ request, env }) {
  const email = await callerEmail(request, env);
  if (!email) return json({ error: "not signed in" }, 401);
  if (!isCurator(email, env)) return json({ error: "only the curator can edit" }, 403);

  let body;
  try { body = await request.json(); } catch { return json({ error: "expected JSON" }, 400); }
  const { id, field, text } = body ?? {};
  if (typeof id !== "string" || !/^entry:[\w.-]{1,200}$/.test(id)) return json({ error: "bad id" }, 400);
  if (field !== "reply" && field !== "context") return json({ error: "bad field" }, 400);
  if (text !== null && (typeof text !== "string" || text.length > MAX_TEXT)) return json({ error: "bad text" }, 400);

  if (text === null) {
    await env.DB.prepare("DELETE FROM edits WHERE entry_id = ? AND field = ?").bind(id, field).run();
  } else {
    await env.DB.prepare(
      `INSERT INTO edits (entry_id, field, text, updated_at, updated_by) VALUES (?, ?, ?, ?, ?)
       ON CONFLICT (entry_id, field) DO UPDATE SET text = excluded.text, updated_at = excluded.updated_at, updated_by = excluded.updated_by`,
    ).bind(id, field, text, new Date().toISOString(), email).run();
  }
  return json({ ok: true });
}
