// Load the page in headless Chrome and report console output, uncaught errors, a few DOM facts,
// and a screenshot. For checking the page without a person at the screen.
//
//   node tools/inspect.mjs [url] [out.png] [seconds]
//
// Uses the Chrome DevTools Protocol directly (Node's built-in WebSocket), no dependencies.

import { spawn } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const url = process.argv[2] ?? "http://127.0.0.1:8086/";
const out = process.argv[3] ?? "page.png";
const seconds = Number(process.argv[4] ?? 12);
// Optional: a script to run in the page after it loads (e.g. clicks), then wait a few seconds.
const action = process.argv[5] ? (await import("node:fs")).readFileSync(process.argv[5], "utf8") : null;
const CHROME = process.env.CHROME ?? "C:/Program Files/Google/Chrome/Application/chrome.exe";
const port = 9300 + Math.floor(Math.random() * 500);

const chrome = spawn(CHROME, [
  "--headless=new", `--remote-debugging-port=${port}`, `--user-data-dir=${mkdtempSync(join(tmpdir(), "aph-chrome-"))}`,
  "--window-size=1600,1000", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--no-first-run", "about:blank",
], { stdio: "ignore" });

const sleep = ms => new Promise(r => setTimeout(r, ms));
let target;
for (let i = 0; i < 50 && !target; i++) {
  await sleep(200);
  try { target = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find(t => t.type === "page"); } catch {}
}
if (!target) { console.error("chrome did not start"); chrome.kill(); process.exit(1); }

const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise(r => ws.addEventListener("open", r, { once: true }));
let id = 0;
const pending = new Map();
const send = (method, params = {}) => new Promise(resolve => { const i = ++id; pending.set(i, resolve); ws.send(JSON.stringify({ id: i, method, params })); });
const log = [];
ws.addEventListener("message", ev => {
  const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m.result ?? m.error); pending.delete(m.id); return; }
  if (m.method === "Runtime.consoleAPICalled") log.push(`console.${m.params.type}: ${m.params.args.map(a => a.value ?? a.description ?? a.type).join(" ")}`);
  if (m.method === "Runtime.exceptionThrown") { const d = m.params.exceptionDetails; log.push(`EXCEPTION: ${d.exception?.description ?? d.text} @ ${d.url ?? ""}:${d.lineNumber}:${d.columnNumber}`); }
  if (m.method === "Log.entryAdded" && m.params.entry.level !== "verbose") log.push(`log.${m.params.entry.level}: ${m.params.entry.text} ${m.params.entry.url ?? ""}`);
});
await send("Runtime.enable");
await send("Log.enable");
await send("Page.enable");
// APH_PHONE=1: emulate a phone (iPhone-14-sized viewport, touch, mobile UA).
if (process.env.APH_PHONE) {
  await send("Emulation.setDeviceMetricsOverride", { width: 390, height: 844, deviceScaleFactor: 3, mobile: true });
  await send("Emulation.setTouchEmulationEnabled", { enabled: true, maxTouchPoints: 5 });
  await send("Emulation.setUserAgentOverride", { userAgent: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1" });
}
await send("Page.navigate", { url });
await sleep(seconds * 1000);
if (action) {
  const r = await send("Runtime.evaluate", { expression: action, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) log.push("ACTION FAILED: " + (r.exceptionDetails.exception?.description ?? r.exceptionDetails.text));
  else if (r.result?.value !== undefined) log.push("ACTION RESULT: " + r.result.value);
  await sleep(4000);
}

const facts = await send("Runtime.evaluate", { returnByValue: true, expression: `(() => ({
  feedMessages: document.querySelectorAll("#feed [data-id]").length,
  members: document.querySelectorAll("#members .mem").length,
  profileHidden: document.getElementById("prof")?.hidden,
  loader: document.querySelector(".globe-loading")?.textContent?.trim() ?? null,
  canvas: (() => { const c = document.getElementById("globeCanvas"); return c ? c.width + "x" + c.height : null; })(),
  overflowX: document.documentElement.scrollWidth - innerWidth,
  viewport: innerWidth + "x" + innerHeight,
  smallTargets: [...document.querySelectorAll("button, a, input, select, [role=button]")].filter(e => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && r.top < innerHeight && (r.width < 32 || r.height < 32); }).slice(0, 25).map(e => (e.id || e.className || e.tagName) + " " + Math.round(e.getBoundingClientRect().width) + "x" + Math.round(e.getBoundingClientRect().height)),
  offscreen: [...document.querySelectorAll("#app > *, #me, #welcome .wcard")].filter(e => { const r = e.getBoundingClientRect(); return r.width > 0 && (r.right > innerWidth + 1 || r.left < -1); }).map(e => e.id || e.className),
  bodyText: document.body.innerText.slice(0, 120),
}))()` });
const shot = await send("Page.captureScreenshot", { format: "png" });
writeFileSync(out, Buffer.from(shot.data, "base64"));
console.log(log.join("\n") || "(no console output)");
console.log(JSON.stringify(facts.result?.value, null, 1));
ws.close();
chrome.kill();
process.exit(0);
