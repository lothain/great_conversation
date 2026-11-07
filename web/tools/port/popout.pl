# v4 request: the globe pops out into a real browser window (globe.js mirrors it), and the
# pane bar's controls can't be text-selected.
sub1(qr{  \.paneBar \{ height: 28px;}, sub { q~  .paneBar { user-select: none; -webkit-user-select: none; height: 28px;~ }, "pane bar no select");
sub1(qr{  \.paneBar button \{ background: none;}, sub { q~  .paneBar button { display: grid; place-items: center; line-height: 0; background: none;~ }, "pane bar button");
my $icon = q{<svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M9 2.5h4.5V7M13.5 2.5 7.5 8.5M11.5 9.5v3.5a.5.5 0 0 1-.5.5H3a.5.5 0 0 1-.5-.5V5a.5.5 0 0 1 .5-.5h3.5"/></svg>};
sub1(qr{<button id="popBtn" title="Pop the globe out">⧉</button>}, sub { qq{<button id="popBtn" title="Open the globe in its own window" aria-label="Open the globe in its own window">$icon</button>} }, "pop icon");
sub1(qr{<div id="poppedNote">The globe is in its own window\. <button class="linkbtn" id="dockBtn2">Dock it here</button></div>},
  sub { q{<div id="poppedNote">The globe is in its own window. <button class="linkbtn" id="dockBtn2">Bring it back</button></div>} }, "popped note");
# The window: same page, only the globe, and the page script stops right after boot.
sub1(qr{(const \{ D, globe \} = await boot\("#globeCanvas"\);\n)}, sub { $1 . q{if (isGlobeWindow) {
  document.title = "The World · The Great Conversation";
  document.body.classList.add("globe-window");
  document.body.appendChild(document.getElementById("globeHost"));
  await new Promise(() => {});   // the opener drives this globe; nothing else runs here
}
} }, "globe window mode");
sub1(qr{import \{ boot \} from "\./globe\.js";}, sub { q{import { boot, isGlobeWindow } from "./globe.js";} }, "import isGlobeWindow");
sub1(qr{<style>\n}, sub { qq{<style>\n  body.globe-window > :not(#globeHost) { display: none !important; }\n  body.globe-window #globeHost { position: fixed; inset: 0; }\n} }, "globe window css");
sub1(qr{const gf = \$\("globeFloat"\);\nfunction popOut\(on\) \{\n.*?\nif \(layout\.popped\) popOut\(true\);\n}s, sub { q{// Pop-out: a real browser window running the same globe (globe.js mirrors every command to it
// and passes its clicks back), so everything behaves as when docked.
function popOut(on) {
  if (on && !globe.popOut()) return;   // the browser blocked the window
  if (!on && globe.popped) globe.dock();
  $("globePane").classList.toggle("popped", on); $("poppedNote").style.display = on ? "block" : "none"; $("hsplitL").style.display = on ? "none" : "";
}
$("popBtn").onclick = () => popOut(true);
$("dockBtn2").onclick = () => popOut(false);
globe.on("docked", () => popOut(false));
} }, "pop out logic");
# A follow-on message's avatar showed on hover squashed to 4px high: show it as a small round
# avatar instead, taking the same width and no extra height, so nothing moves.
sub1(qr{  \.msg\.cont:hover \.av\.small \{ visibility: visible; \}\n}, sub { "  .msg.cont:hover .av.small { visibility: visible; width: 26px; height: 26px; margin: 0 9px -22px; font-size: 11px; }\n" }, "follow-on avatar");
