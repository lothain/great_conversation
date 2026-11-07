// Trunk post_build hook: stamp the files Trunk doesn't hash, so a new build is never mixed with
// cached pieces of the old one. Trunk already hashes the WASM and its JS glue.
//   index.html  ./globe.js and ./account.js get ?v=<hash of the build's content>
//   globe.js    passes its own ?v= on to every data fetch (see the top of globe.js)
//   _headers    Cloudflare Pages: hashed files cached forever; everything else revalidated
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const dir = process.env.TRUNK_STAGING_DIR;
if (!dir) { console.error("stamp.mjs runs as a Trunk post_build hook"); process.exit(2); }

const hash = createHash("sha256");
(function walk(d) {
  for (const f of readdirSync(d).sort()) {
    const p = join(d, f);
    if (statSync(p).isDirectory()) walk(p); else if (f !== "index.html") hash.update(f).update(readFileSync(p));
  }
})(dir);
const v = hash.digest("hex").slice(0, 12);

const page = join(dir, "index.html");
let html = readFileSync(page, "utf8");
const before = html;
html = html.replace(/(["'])\.\/(globe|account)\.js\1/g, (_, q, name) => `${q}./${name}.js?v=${v}${q}`);
if (html === before) { console.error("stamp.mjs: no ./globe.js or ./account.js import found"); process.exit(1); }

// The WASM download: start it only when the globe is actually on screen (a phone may never open
// it), and report progress for the loading state. Trunk's loader calls init(path); hand it a
// Response whose body we stream ourselves, counting bytes, once globe.js says "go".
const wasmRef = html.match(/init\(\{ module_or_path: '([^']+\.wasm)' \}\)/);
if (!wasmRef) { console.error("stamp.mjs: Trunk's init(...) call not found"); process.exit(1); }
const wasmPath = wasmRef[1];
const wasmBytes = statSync(join(dir, wasmPath.replace(/^\//, ""))).size;
html = html
  .replace(wasmRef[0], `init({ module_or_path: window.__aphWasm(${JSON.stringify(wasmPath)}, ${wasmBytes}) })`)
  .replace(/<link rel="preload" href="[^"]+\.wasm"[^>]*>/, "")
  .replace("<head>", `<head>
<script>
// Set by tools/stamp.mjs. globe.js calls __aphStartGlobe() once the globe is visible; until then
// the WASM isn't fetched. Progress goes out as "aph-wasm-progress" events ({ loaded, total }).
window.__aphGo = new Promise(go => { window.__aphStartGlobe = go; });
window.__aphWasm = async (path, total) => {
  await window.__aphGo;
  const r = await fetch(path);
  if (!r.ok || !r.body) return r;
  let loaded = 0;
  const say = () => dispatchEvent(new CustomEvent("aph-wasm-progress", { detail: { loaded, total } }));
  say();
  const counted = r.body.pipeThrough(new TransformStream({ transform(chunk, c) { loaded += chunk.byteLength; say(); c.enqueue(chunk); } }));
  return new Response(counted, { headers: { "content-type": "application/wasm" } });
};
</script>`);
writeFileSync(page, html);

writeFileSync(join(dir, "_headers"), `/*
  Cache-Control: no-cache
/aph-globe-*
  ! Cache-Control
  Cache-Control: public, max-age=31536000, immutable
`);
console.log("stamped build", v);
