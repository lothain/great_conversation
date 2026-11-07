// ---------- globe: Bevy/WASM via globe.js (replaces globe.gl, the parchment canvas and DOM dots) ----------
const globeEl = $("globe"), globeHost = $("globeHost"), keys = {};
// The canvas follows its container on its own (pop-out, splitters); nothing to fit by hand.
const fit = () => {};
if (saved?.pov) globe.pointOfView(saved.pov);
// The globe shows no popups. A click (not a drag) reports the dots under the pointer: none
// closes the profile, like clicking anywhere else; one dot opens its
// speaker's profile with that utterance selected; several by one speaker open the profile
// without choosing; several speakers is ambiguous, so nothing happens. (Double-click zooms in
// to separate them; the globe does that itself.)
globe.on("picked", e => {
  const said = e.nodes.map(i => D.nodes[i]).filter(Boolean);
  if (!said.length) { if (prof.open) closeProfile(); return; }   // clicked the globe itself: click away
  const speakers = [...new Set(said.map(n => n.author))];
  if (speakers.length !== 1) return;
  const who = speakers[0];
  openProfile(who);
  if (said.length === 1) {
    prof.sel[who] = said[0].id; prof.tab = "utt"; keys.prof = null; updateProfile();
    focusMsg(said[0].id);
  }
});
globe.on("hovered", e => { globeEl.style.cursor = e.overDot ? "pointer" : ""; });
globe.on("view", () => saveSoon());

