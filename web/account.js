// Who is listening, and the curator's edits.
//
// Sign-in is Cloudflare Access (email one-time PIN) in front of /auth/*. The page itself is
// public: anyone can read; signing in lets a reader set a display name, and lets the curator
// (an allow-listed email, checked on the server) edit the curator's replies and contexts.
//
//   await account.load()                 public edits (everyone sees the same text)
//   account.me                           null | { email, displayName, curator }
//   account.onChange(fn)                 called when sign-in state or edits change
//   account.sync("reply" | "context", map)   save changes to a map of id -> text (curator only)
//   account.signIn() / signOut() / setDisplayName(name)
//
// Without the Cloudflare backend (e.g. `trunk serve`) the API is absent: no edits, nobody signed in.

const listeners = [];
const changed = () => { for (const fn of listeners) fn(); };

// What the server holds, per field, so sync() sends only differences.
const saved = { reply: {}, context: {} };
const timers = {};

function notice(text) {
  let el = document.getElementById("account-notice");
  if (!el) {
    el = Object.assign(document.createElement("div"), { id: "account-notice", role: "status" });
    Object.assign(el.style, {
      position: "fixed", left: "50%", bottom: "64px", transform: "translateX(-50%)", zIndex: 60,
      background: "#3b2f22", color: "#efe4c8", padding: "8px 14px", borderRadius: "8px",
      font: "15px 'EB Garamond', Georgia, serif", boxShadow: "0 6px 20px rgba(0,0,0,.35)",
    });
    document.body.appendChild(el);
  }
  el.textContent = text;
  el.hidden = false;
  clearTimeout(notice.t);
  notice.t = setTimeout(() => { el.hidden = true; }, 5000);
}

const typingListeners = [];
let lastTyping = 0;

async function write(method, body) {
  const r = await json("/auth/api/posts", { method, headers: { "content-type": "application/json" }, body: JSON.stringify(body) })
    .catch(e => ({ error: e.message }));
  if (r.signedOut) { notice("Please sign in again."); return false; }
  if (r.error) { notice(r.error.includes("429") ? "Slow down a little." : "Couldn't save your message."); return false; }
  await account.loadPosts();
  return true;
}

// The live socket: the server says when messages change (refetch) and who is typing.
// Reconnects with backoff; without the backend (trunk serve) it quietly gives up. Phones pause
// sockets in the background without closing them, so coming back to the page always refetches
// and reconnects if the socket is gone, and a slow poll covers anything still missed.
let liveSocket = null, liveGaveUp = false, lastHeard = 0;
function connectLive(delay = 1000) {
  if (!("WebSocket" in window) || liveGaveUp) return;
  const ws = liveSocket = new WebSocket(`${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/api/live`);
  let opened = false, ping = 0;
  ws.onopen = () => { opened = true; delay = 1000; lastHeard = Date.now(); ping = setInterval(() => ws.readyState === 1 && ws.send("ping"), 30000); };
  ws.onmessage = ev => {
    lastHeard = Date.now();
    if (ev.data === "pong") return;
    let m; try { m = JSON.parse(ev.data); } catch { return; }
    if (m.type === "posts") account.loadPosts();
    if (m.type === "typing" && m.author !== account.me?.id) for (const fn of typingListeners) fn(m);
  };
  ws.onclose = () => {
    clearInterval(ping);
    if (liveSocket !== ws) return;               // replaced by a newer socket
    if (!opened && delay >= 8000) { liveGaveUp = true; return; }   // never connected: no live backend
    setTimeout(() => { if (liveSocket === ws) connectLive(Math.min(delay * 2, 120000)); }, delay);
  };
}
function catchUp() {
  if (liveGaveUp) return;
  account.loadPosts();
  // No pong for over a minute means the socket died quietly (phone asleep): start a fresh one.
  if (!liveSocket || liveSocket.readyState > 1 || Date.now() - lastHeard > 65000) { try { liveSocket?.close(); } catch {} liveSocket = null; connectLive(); }
}
document.addEventListener("visibilitychange", () => { if (!document.hidden) catchUp(); });
addEventListener("pageshow", e => { if (e.persisted) catchUp(); });
setInterval(() => { if (!document.hidden) catchUp(); }, 120000);

async function json(url, init = {}) {
  const r = await fetch(url, { credentials: "same-origin", redirect: "manual", ...init });
  // Access answers unauthenticated /auth/* requests with a redirect to its login page.
  if (r.type === "opaqueredirect" || r.status === 401 || r.status === 403) return { signedOut: true, status: r.status };
  if (!r.ok) throw new Error(`${url}: ${r.status} ${await r.text().catch(() => "")}`.trim());
  return r.json();
}

export const account = {
  me: null,
  edits: { reply: {}, context: {} },

  onChange(fn) { listeners.push(fn); },

  async load() {
    try {
      const data = await Promise.race([json("/api/edits"), new Promise((_, no) => setTimeout(() => no(new Error("timeout")), 4000))]);
      for (const field of ["reply", "context"]) {
        Object.assign(account.edits[field], data[field] || {});
        Object.assign(saved[field], data[field] || {});
      }
    } catch (e) {
      console.info("curator edits unavailable:", e.message);
    }
    // Who's signed in doesn't block the page.
    const tried = sessionStorage.getItem("aph.signingIn");
    sessionStorage.removeItem("aph.signingIn");
    json("/auth/api/me")
      .then(me => {
        account.me = me.signedOut ? null : me;
        // Back from a sign-in that didn't take (e.g. Cloudflare Access isn't set up for /auth/*).
        if (tried && !account.me) notice("Sign-in didn't complete. Please try again.");
        changed();
        account.loadPosts();          // "mine" depends on who is signed in
      })
      .catch(() => { if (tried) notice("Sign-in isn't available right now."); account.loadPosts(); });
    connectLive();
    return account;
  },

  signIn() {
    sessionStorage.setItem("aph.signingIn", "1");
    // Access shows its email + one-time PIN page, then /auth/login sends the reader back here.
    location.href = "/auth/login?return=" + encodeURIComponent(location.pathname + location.search + location.hash);
  },

  signOut() {
    location.href = "/cdn-cgi/access/logout";
  },

  async setDisplayName(name) {
    const r = await json("/auth/api/name", { method: "PUT", headers: { "content-type": "application/json" }, body: JSON.stringify({ displayName: name }) })
      .catch(e => ({ error: e.message }));
    if (r.error || r.signedOut) {
      notice(r.signedOut ? "Please sign in first." : (r.error.includes("409") ? "That name is taken." : "Couldn't save your name."));
      return false;
    }
    account.me = { ...account.me, displayName: r.displayName };
    changed();
    return true;
  },

  // ---- readers' messages (shared through the server) ----
  posts: [],                                  // { id, channel, thread, text, at, edited, author, authorName, mine }

  async loadPosts() {
    try {
      const r = await json("/api/posts");
      account.posts = (r.posts || []).map(p => ({ ...p, mine: !!account.me && p.author === account.me.id }));
    } catch (e) {
      console.info("posts unavailable:", e.message);
    }
    changed();
  },

  async post(channel, thread, text) {
    if (!account.me) { notice("Sign in (your avatar, top right) to join the conversation."); return false; }
    return write("POST", { channel, thread, text });
  },
  editPost(id, text) { return write("PUT", { id, text }); },
  deletePost(id) { return write("DELETE", { id }); },

  // ---- live: new messages and who is typing ----
  onTyping(fn) { typingListeners.push(fn); },
  /// Call as the reader types; tells others at most every 3 seconds.
  typing(channel, thread) {
    if (!account.me || Date.now() - lastTyping < 3000) return;
    lastTyping = Date.now();
    fetch("/auth/api/typing", { method: "POST", credentials: "same-origin", redirect: "manual",
      headers: { "content-type": "application/json" }, body: JSON.stringify({ channel, thread }) }).catch(() => {});
  },

  /// Save what changed in `map` (id -> text; missing ids are reverted to the original).
  sync(field, map) {
    if (!account.me?.curator) return;
    clearTimeout(timers[field]);
    timers[field] = setTimeout(async () => {
      const was = saved[field];
      const ops = [
        ...Object.entries(map).filter(([id, text]) => was[id] !== text).map(([id, text]) => ({ id, text })),
        ...Object.keys(was).filter(id => !(id in map)).map(id => ({ id, text: null })),
      ];
      for (const { id, text } of ops) {
        try {
          const r = await json("/auth/api/edits", { method: "PUT", headers: { "content-type": "application/json" }, body: JSON.stringify({ id, field, text }) });
          if (r.signedOut) throw new Error("signed out");
          if (text === null) delete was[id]; else was[id] = text;
        } catch (e) {
          console.error("saving edit failed:", e);
          notice("Couldn't save that edit. Are you still signed in?");
          return;
        }
      }
    }, field === "context" ? 600 : 0);
  },
};
