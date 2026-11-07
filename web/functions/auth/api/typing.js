// POST /auth/api/typing { channel, thread? }: "<your name> is typing…" for everyone else.
// Signed in only, and the name comes from the server, so no one can type as someone else.
import { callerEmail, json } from "../../_lib/access.js";
import { announce, readerId } from "../../_lib/live.js";

export async function onRequestPost({ request, env }) {
  const email = await callerEmail(request, env);
  if (!email) return json({ error: "not signed in" }, 401);
  let b; try { b = await request.json(); } catch { return json({ error: "expected JSON" }, 400); }
  if (typeof b?.channel !== "string" || b.channel.length > 120) return json({ error: "bad request" }, 400);
  const row = await env.DB.prepare("SELECT display_name FROM users WHERE email = ?").bind(email).first();
  const name = row?.display_name;
  if (!name) return json({ ok: true });   // no display name yet: nothing to show others
  await announce(env, { type: "typing", channel: b.channel, thread: typeof b.thread === "string" ? b.thread : null, author: await readerId(email), name });
  return json({ ok: true });
}
