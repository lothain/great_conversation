// The Bevy globe, behind a small API in the vocabulary the page already used with globe.gl.
//
//   const { D, globe } = await boot("#globeCanvas");   // resolves as soon as the page model is in
//   globe.setTime(year, window)          years; window 0 = everything so far
//   globe.pointOfView()                  → { lat, lng, altitude }   (altitude in globe radii above the surface)
//   globe.pointOfView({lat, lng, altitude?}, ms)   ease the camera there
//   globe.highlight(nodeId | null)       ring one node's dot
//   globe.civMask(Set | null)            emphasize only these civilizations
//   globe.religionMask(Set | null)       dim dots of other religions
//   globe.popOut() / globe.dock()       the globe in its own browser window (see "pop-out" below)
//   globe.on("ready" | "picked" | "hovered" | "view", fn)
//     picked:  { nodes: [index…] }  dots under the pointer, nearest first (indices into D.nodes)
//     hovered: { overDot }          for the cursor; the globe shows no popups
//     docked:  {}                   the pop-out window closed
//
// The page never waits for the globe: the page model is baked (`data/baked/page-model.json`),
// the WASM module starts only after the page has painted, and until the globe is ready its
// commands are queued (latest of each kind wins) while a loading state covers the globe pane.

// Everything is cached in data/ and served with the page; nothing is fetched from elsewhere.
// The build stamps this module's URL (?v=...); pass the stamp on so data is never stale.
const V = new URL(import.meta.url).search;
const PAGE_MODEL = "data/baked/page-model.json" + V;
const GLOBE_DATA = ["aphorisms.json", "graph.json", "civilizations.geojson", "portraits.json"];
const COASTS = "data/baked/coasts.png" + V;

// Pop-out: a real browser window (index.html#globe-window) runs its own globe. The page keeps
// talking to its `globe` object exactly as before: every command is mirrored to the window, and
// the window's picked / hovered / view events come back as this globe's events. So clicks,
// double clicks, hover, highlight, time and masks behave the same docked or popped.
export const isGlobeWindow = location.hash === "#globe-window" && !!window.opener;
const WINDOW_NAME = "aph-globe";

export async function boot(canvasSelector) {
  const D = await fetch(PAGE_MODEL).then(r => {
    if (!r.ok) throw new Error(`${PAGE_MODEL}: ${r.status} (run the bake step: trunk build does it)`);
    return r.json();
  });
  return { D, globe: createGlobe(D, canvasSelector) };
}

function wasmReady() {
  return new Promise(resolve => {
    if (window.wasmBindings) resolve(window.wasmBindings);
    else addEventListener("TrunkApplicationStarted", () => resolve(window.wasmBindings), { once: true });
  });
}

const afterFirstPaint = () => new Promise(resolve => requestAnimationFrame(() => setTimeout(resolve, 0)));

const LOADER_CSS = `
.globe-loading { position: absolute; inset: 0; display: grid; place-items: center; z-index: 2;
  background: radial-gradient(circle at 50% 45%, #3a332b 0%, #2a2520 70%); color: #efe4c8;
  font: italic 15px "EB Garamond", Georgia, serif; transition: opacity .5s ease; pointer-events: none; }
.globe-loading.done { opacity: 0; }
.globe-loading .orb { width: 92px; height: 92px; margin: 0 auto 14px; border-radius: 50%;
  background: radial-gradient(circle at 35% 30%, #efe4c8, #b9a67c 60%, #6b5a44);
  box-shadow: 0 0 24px rgba(217,199,156,.35); animation: globe-turn 2.4s ease-in-out infinite; }
.globe-loading .what { opacity: .75; min-height: 1.3em; text-align: center; }
.globe-loading .bar { width: 180px; height: 3px; margin: 10px auto 0; border-radius: 2px; background: rgba(239,228,200,.18); overflow: hidden; }
.globe-loading .bar[hidden] { display: none; }
.globe-loading .bar i { display: block; height: 100%; width: 0; background: #d9c79c; transition: width .2s linear; }
.globe-loading.failed .orb { animation: none; filter: grayscale(1) brightness(.6); }
@keyframes globe-turn { 0%, 100% { transform: scale(.94); opacity: .75 } 50% { transform: scale(1); opacity: 1 } }
@media (prefers-reduced-motion: reduce) { .globe-loading .orb { animation: none; } }
`;

function showLoader(canvas) {
  if (!document.getElementById("globe-loading-css")) {
    document.head.insertAdjacentHTML("beforeend", `<style id="globe-loading-css">${LOADER_CSS}</style>`);
  }
  const el = document.createElement("div");
  el.className = "globe-loading";
  el.setAttribute("role", "status");
  el.innerHTML = `<div><div class="orb" aria-hidden="true"></div><div class="what">Loading the globe…</div><div class="bar" hidden><i></i></div></div>`;
  canvas.parentElement.appendChild(el);
  return {
    say: text => { el.querySelector(".what").textContent = text; },
    progress: (fraction, text) => {
      el.querySelector(".what").textContent = text;
      const bar = el.querySelector(".bar");
      bar.hidden = false;
      bar.firstElementChild.style.width = (Math.max(0, Math.min(1, fraction)) * 100).toFixed(1) + "%";
    },
    done: () => { el.classList.add("done"); setTimeout(() => el.remove(), 600); },
    fail: text => { el.classList.add("failed"); el.querySelector(".what").textContent = text; },
  };
}

function createGlobe(D, canvasSelector) {
  const index = new Map(D.nodes.map((n, i) => [n.id, i]));
  const religionBit = new Map(D.colors.religion_order.map((f, i) => [f, i]));
  const listeners = {};
  const emit = (type, e) => { for (const fn of listeners[type] || []) fn(e); };

  let wasm = null;
  const pending = new Map();   // command type -> latest command, until the globe is up
  const latest = new Map();     // command type -> latest command, replayed to a new pop-out window
  let remote = null, remoteWatch = 0;
  const send = command => {
    if (command.type !== "focus") latest.set(command.type, command);
    if (remote && !remote.closed) remote.postMessage({ aph: "cmd", command }, location.origin);
    return wasm ? wasm.send(JSON.stringify(command)) : pending.set(command.type, command);
  };

  let view = { lat: 38, lng: 25, altitude: 2.1 };
  let lastTime = null, lastHighlight = null;

  const globe = {
    ready: false,
    on(type, fn) { (listeners[type] ||= []).push(fn); return globe; },
    setTime(year, window = 0) {
      const key = year + "|" + window;
      if (key !== lastTime) { lastTime = key; send({ type: "setTime", year, window }); }
    },
    pointOfView(pov) {
      if (!pov) return view;
      view = { ...view, ...pov };
      send({ type: "focus", lat: pov.lat, lon: pov.lng, distance: pov.altitude == null ? null : 1 + pov.altitude });
      return globe;
    },
    highlight(id) {
      const node = id == null ? null : index.get(id) ?? null;
      if (node !== lastHighlight) { lastHighlight = node; send({ type: "highlight", node }); }
    },
    civMask(set) { send({ type: "civMask", civs: set ? [...set] : null }); },
    religionMask(set) {
      if (!set || !set.size) return send({ type: "religionMask", bits: null });
      let bits = 0;
      for (const f of set) if (religionBit.has(f)) bits |= 1 << religionBit.get(f);
      send({ type: "religionMask", bits: bits >>> 0 });
    },
    get popped() { return !!remote; },
    popOut() {
      if (isGlobeWindow) return false;
      if (remote && !remote.closed) { remote.focus(); return true; }
      const r = canvas.getBoundingClientRect();
      const w = Math.max(520, Math.round(r.width)), h = Math.max(480, Math.round(r.height));
      remote = window.open(location.pathname + location.search + "#globe-window", WINDOW_NAME, `popup,width=${w},height=${h}`);
      if (!remote) return false;   // blocked
      clearInterval(remoteWatch);
      remoteWatch = setInterval(() => { if (remote?.closed) docked(); }, 400);
      return true;
    },
    dock() { if (remote && !remote.closed) remote.close(); docked(); },
  };

  function docked() {
    if (!remote) return;
    remote = null; clearInterval(remoteWatch);
    send({ type: "focus", lat: view.lat, lon: view.lng, distance: 1 + view.altitude });   // pick up where the window was
    emit("docked", {});
  }
  addEventListener("message", m => {
    if (m.origin !== location.origin || !m.data?.aph) return;
    const { aph } = m.data;
    if (isGlobeWindow) {
      if (m.source === window.opener && aph === "cmd") send(m.data.command);
      return;
    }
    if (!remote || m.source !== remote) return;
    if (aph === "hello") {
      for (const command of latest.values()) remote.postMessage({ aph: "cmd", command }, location.origin);
      remote.postMessage({ aph: "cmd", command: { type: "focus", lat: view.lat, lon: view.lng, distance: 1 + view.altitude } }, location.origin);
    } else if (aph === "event") {
      const e = m.data.event;
      if (e.type === "view") view = { lat: e.lat, lng: e.lon, altitude: e.distance - 1 };
      if (e.type !== "ready") emit(e.type, e);
    } else if (aph === "bye") docked();
  });
  if (isGlobeWindow) {
    window.opener.postMessage({ aph: "hello" }, location.origin);
    addEventListener("pagehide", () => window.opener?.postMessage({ aph: "bye" }, location.origin));
  } else {
    addEventListener("pagehide", () => remote?.close());
  }

  const canvas = document.querySelector(canvasSelector);
  const loader = showLoader(canvas);

  (async () => {
    await afterFirstPaint();
    // Only download the globe once it's on screen (on a phone it's behind the World button).
    await new Promise(resolve => {
      const visible = () => canvas.getClientRects().length > 0 && canvas.clientWidth > 0 && canvas.clientHeight > 0;
      if (visible()) return resolve();
      const ro = new ResizeObserver(() => { if (visible()) { ro.disconnect(); resolve(); } });
      ro.observe(canvas);
    });
    addEventListener("aph-wasm-progress", e => {
      const { loaded, total } = e.detail, mb = n => (n / 1048576).toFixed(1);
      if (loaded < total) loader.progress(loaded / total, `Downloading the globe… ${mb(loaded)} of ${mb(total)} MB`);
      else loader.progress(1, "Starting the globe…");
    });
    window.__aphStartGlobe?.();
    const [bindings, texts, coasts] = await Promise.all([
      wasmReady(),
      Promise.all(GLOBE_DATA.map(f => fetch("data/" + f + V).then(r => r.ok ? r.text() : null))),
      fetch(COASTS).then(r => r.ok ? r.arrayBuffer() : null).then(b => b && new Uint8Array(b)).catch(() => null),
    ]);
    loader.say("Drawing the world…");
    await afterFirstPaint();
    bindings.on_event(json => {
      const e = JSON.parse(json);
      if (e.type === "ready") { globe.ready = true; loader.done(); }
      if (isGlobeWindow) { window.opener?.postMessage({ aph: "event", event: e }, location.origin); return; }
      if (remote && e.type !== "ready") return;   // popped: the window speaks for the globe
      if (e.type === "view") view = { lat: e.lat, lng: e.lon, altitude: e.distance - 1 };
      emit(e.type, e);
    });
    const [aphorisms, graph, civs, portraits] = texts;
    bindings.start_globe(canvasSelector, aphorisms, graph, civs, portraits, null, coasts);
    wasm = bindings;
    for (const command of pending.values()) send(command);
    pending.clear();
  })().catch(err => {
    console.error("globe failed to start:", err);
    loader.fail("The globe couldn't load. Everything else still works.");
  });

  return globe;
}
