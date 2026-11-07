// GET /api/edits: Tristan's edits, public, so every reader sees the same text.
// -> { reply: { "entry:<id>": text, ... }, context: { ... } }

import { json } from "../_lib/access.js";

export async function onRequestGet({ env }) {
  const { results } = await env.DB.prepare("SELECT entry_id, field, text FROM edits").all();
  const out = { reply: {}, context: {} };
  for (const row of results) out[row.field][row.entry_id] = row.text;
  return json(out);
}
