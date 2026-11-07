// GET /api/posts: every reader's message, for everyone. Emails are never sent: each post
// carries an opaque author id (the same one /auth/api/me returns as `id`) and its author's
// display name (everyone who posts has one).
import { json } from "../_lib/access.js";
import { readerId } from "../_lib/live.js";

export async function onRequestGet({ env }) {
  const { results } = await env.DB.prepare(
    `SELECT p.id, p.channel, p.thread, p.text, p.author, p.at, p.edited, u.display_name
       FROM posts p LEFT JOIN users u ON u.email = p.author ORDER BY p.at LIMIT 5000`,
  ).all();
  const ids = new Map();
  const posts = [];
  for (const r of results) {
    if (!ids.has(r.author)) ids.set(r.author, await readerId(r.author));
    posts.push({ id: r.id, channel: r.channel, thread: r.thread, text: r.text, at: r.at, edited: r.edited,
      author: ids.get(r.author), authorName: r.display_name });
  }
  return json({ posts });
}
