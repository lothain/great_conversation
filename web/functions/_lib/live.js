// Tell everyone connected to the live socket that something happened (see live/worker.js).
// Best effort: a missing binding (e.g. local dev without the live worker) or a failure never
// fails the request that caused it.
export async function announce(env, message) {
  try {
    if (!env.LIVE) return;
    const room = env.LIVE.get(env.LIVE.idFromName("all"));
    await room.fetch("https://live/announce", { method: "POST", body: JSON.stringify(message) });
  } catch (e) {
    console.warn("live announce failed:", e?.message);
  }
}

/// A stable, opaque id for a reader: lets the page tell "mine" apart without seeing emails.
export async function readerId(email) {
  const d = await crypto.subtle.digest("SHA-256", new TextEncoder().encode("gc-reader:" + email));
  return [...new Uint8Array(d)].slice(0, 9).map(b => b.toString(16).padStart(2, "0")).join("");
}
