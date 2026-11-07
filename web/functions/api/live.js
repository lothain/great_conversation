// GET /api/live (WebSocket): the live socket. Anyone may listen; only the server speaks
// (announcements come from the post and typing APIs), so listeners can't impersonate anyone.
import { json } from "../_lib/access.js";

export async function onRequestGet({ request, env }) {
  if (request.headers.get("Upgrade") !== "websocket") return json({ error: "expected a WebSocket" }, 426);
  if (!env.LIVE) return json({ error: "live updates aren't configured" }, 503);
  const room = env.LIVE.get(env.LIVE.idFromName("all"));
  return room.fetch(new Request("https://live/connect", request));
}
