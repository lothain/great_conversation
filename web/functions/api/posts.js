// GET /api/posts: every reader's message, for everyone. Emails are never sent: each post
// carries an opaque author id (the same one /auth/api/me returns as `id`) and a display name.
// The curator's posts go out under the curator's name.
import { isCurator, json } from "../_lib/access.js";
import { readerId } from "../_lib/live.js";

const CURATOR_NAME = "Tristan";

export async function onRequestGet({ env }) {
  const { results } = await env.DB.prepare(
    `SELECT p.id, p.channel, p.thread, p.text, p.author, p.at, p.edited, u.display_name
       FROM posts p LEFT JOIN users u ON u.email = p.author ORDER BY p.at LIMIT 5000`,
  ).all();
  const ids = new Map();
  const posts = [];
  for (const r of results) {
    if (!ids.has(r.author)) ids.set(r.author, await readerId(r.author));
    const curator = isCurator(r.author, env);
    posts.push({ id: r.id, channel: r.channel, thread: r.thread, text: r.text, at: r.at, edited: r.edited,
      author: ids.get(r.author), authorName: curator ? CURATOR_NAME : r.display_name, curator });
  }
  return json({ posts });
}
