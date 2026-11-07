// Readers' messages, signed in (Cloudflare Access):
//   POST   { channel, thread?, text }   -> { id }
//   PUT    { id, text }                 your own message
//   DELETE { id }                       your own message
// Every change is announced on the live socket so open pages refetch.
import { callerEmail, json } from "../../_lib/access.js";
import { announce } from "../../_lib/live.js";

const MAX = 4000;
const clean = t => (typeof t === "string" ? t.replace(/\r\n?/g, "\n").trim() : "");
const ok = (v, max = 120) => typeof v === "string" && v.length > 0 && v.length <= max && !/[\p{C}]/u.test(v);
async function body(request) { try { return await request.json(); } catch { return null; } }

export async function onRequestPost({ request, env }) {
  const email = await callerEmail(request, env);
  if (!email) return json({ error: "not signed in" }, 401);
  const b = await body(request);
  const text = clean(b?.text);
  if (!ok(b?.channel) || (b.thread != null && !ok(b.thread)) || !text || text.length > MAX) return json({ error: "bad message" }, 400);
  // Everyone who speaks has a display name (Tristan included).
  const named = await env.DB.prepare("SELECT display_name FROM users WHERE email = ? AND display_name IS NOT NULL").bind(email).first();
  if (!named) return json({ error: "choose a display name first" }, 409);
  // A light flood guard: at most 10 messages a minute per reader.
  const since = new Date(Date.now() - 60_000).toISOString();
  const recent = await env.DB.prepare("SELECT COUNT(*) AS n FROM posts WHERE author = ? AND at > ?").bind(email, since).first();
  if (recent.n >= 10) return json({ error: "slow down" }, 429);
  const id = "p" + crypto.randomUUID().replace(/-/g, "").slice(0, 16);
  await env.DB.prepare("INSERT INTO posts (id, channel, thread, text, author, at) VALUES (?, ?, ?, ?, ?, ?)")
    .bind(id, b.channel, b.thread ?? null, text, email, new Date().toISOString()).run();
  await announce(env, { type: "posts", channel: b.channel, thread: b.thread ?? null });
  return json({ id });
}

export async function onRequestPut({ request, env }) {
  const email = await callerEmail(request, env);
  if (!email) return json({ error: "not signed in" }, 401);
  const b = await body(request);
  const text = clean(b?.text);
  if (!ok(b?.id) || !text || text.length > MAX) return json({ error: "bad message" }, 400);
  const row = await env.DB.prepare("SELECT channel, thread FROM posts WHERE id = ? AND author = ?").bind(b.id, email).first();
  if (!row) return json({ error: "not your message" }, 403);
  await env.DB.prepare("UPDATE posts SET text = ?, edited = ? WHERE id = ?").bind(text, new Date().toISOString(), b.id).run();
  await announce(env, { type: "posts", channel: row.channel, thread: row.thread });
  return json({ ok: true });
}

export async function onRequestDelete({ request, env }) {
  const email = await callerEmail(request, env);
  if (!email) return json({ error: "not signed in" }, 401);
  const b = await body(request);
  if (!ok(b?.id)) return json({ error: "bad request" }, 400);
  const row = await env.DB.prepare("SELECT channel, thread FROM posts WHERE id = ? AND author = ?").bind(b.id, email).first();
  if (!row) return json({ error: "not your message" }, 403);
  await env.DB.prepare("DELETE FROM posts WHERE id = ?").bind(b.id).run();
  await announce(env, { type: "posts", channel: row.channel, thread: row.thread });
  return json({ ok: true });
}
