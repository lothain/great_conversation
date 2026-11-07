# v4 request: no "be Tristan". Signing in (Cloudflare Access, email PIN) is the only way to edit,
# only the curator's emails can, edits are shared through the server, and the curator is simply Tristan.

sub1(qr{import \{ boot \} from "\./globe\.js";\nconst \{ D, globe \} = await boot\("#globeCanvas"\);\n},
  sub { qq{import { boot } from "./globe.js";\nimport { account } from "./account.js";\nconst { D, globe } = await boot("#globeCanvas");\nawait account.load();\n} }, "import account");
sub1(qr{  <button role="menuitem" id="beTristan">Be Tristan</button>\n},
  sub { qq{  <button role="menuitem" id="signIn">Sign in</button>\n  <button role="menuitem" id="setName" hidden>Set display name…</button>\n  <button role="menuitem" id="signOut" hidden>Sign out</button>\n} }, "menu items");
sub1(qr{<div class="sub">Settings are kept in this browser</div>}, sub { q{<div class="sub" id="mmSub">Not signed in</div>} }, "menu sub");
sub1(qr{          <label class="row" style="font-size:15\.5px"><input type="checkbox" id="curMode">[^\n]*\n          <p class="sub">[^\n]*\n},
  sub { qq{          <p id="curStatus"></p>\n          <p class="sub">In a thread, hover one of your replies and choose ✎ Edit (Ctrl+Enter saves, Esc cancels). Contexts are edited in a speaker's profile and save as you type. Edits are saved for everyone.</p>\n} }, "settings curator pane");
sub1(qr{ <button id="curDiscard" class="ghostbtn">Discard all edits</button>}, "", "discard button");
sub1(qr{          <p class="sub">To merge downloaded edits into the data:[^\n]*\n}, "", "python hint");

sub1(qr{let CUR_EDITS = load\("gc\.curEdits", \{\}\);[^\n]*\n}, sub { "let CUR_EDITS = account.edits.reply;     // shared: loaded from the server (account.js)\n" }, "reply edits");
sub1(qr{let curatorMode = !!load\("gc\.curatorMode", false\);\n}, sub { "let curatorMode = false;                 // true only when signed in as the curator\n" }, "curator mode");
sub1(qr{let CTX_EDITS = load\("gc\.ctxEdits", \{\}\);[^\n]*\n}, sub { "let CTX_EDITS = account.edits.context;   // shared: loaded from the server (account.js)\n" }, "context edits");
suball(qr{save\("gc\.curEdits", CUR_EDITS\)}, sub { q{account.sync("reply", CUR_EDITS)} }, "save replies");
suball(qr{save\("gc\.ctxEdits", CTX_EDITS\)}, sub { q{account.sync("context", CTX_EDITS)} }, "save contexts");
sub1(qr{  const iam = e\.target\.closest\("\[data-iam\]"\);\n  if \(iam\) \{[^\n]*\n}, "", "this-is-me");
sub1(qr{\$\("curDiscard"\)\.onclick = [^\n]*\n}, "", "discard handler");
sub1(qr{\$\("curMode"\)\.onchange = [^\n]*\n}, "", "curator toggle");
sub1(qr{function setTristan\(on\) \{[^\n]*\n\$\("beTristan"\)\.onclick = [^\n]*\n}, sub { q{// Sign-in is Cloudflare Access (email one-time PIN); the server decides who may edit.
$("signIn").onclick = () => account.signIn();
$("signOut").onclick = () => account.signOut();
$("setName").onclick = async () => {
  meMenu.hidden = true;
  const name = prompt("Your display name (2–40 characters, unique):", account.me?.displayName || "");
  if (name !== null && name.trim()) await account.setDisplayName(name);
};
account.onChange(() => {
  curatorMode = !!account.me?.curator;
  syncCuratorUI(); renderThread(); refreshSlots(); keys.prof = null; updateProfile();
});
} }, "sign-in wiring");
sub1(qr{function syncCuratorUI\(\) \{\n.*?\n\}\n}s, sub { q{function syncCuratorUI() {
  const me = account.me, who = me ? (me.curator ? CURATOR : me.displayName || me.email) : "Guest listener";
  const nr = Object.keys(CUR_EDITS).length, nc = Object.keys(CTX_EDITS).length, n = nr + nc;
  $("curStatus").textContent = curatorMode ? `Signed in as ${CURATOR} (${me.email}): your edits are saved for everyone.`
    : me ? `Signed in as ${me.email}. Only ${CURATOR} can edit.` : `Sign in from your avatar menu (top right) as ${CURATOR} to edit.`;
  $("curEditCount").textContent = n ? `Edited: ${nr} repl${nr === 1 ? "y" : "ies"}, ${nc} context${nc === 1 ? "" : "s"}.` : "No edits yet.";
  $("curExport").disabled = !n;
  document.querySelector("#me .mename").textContent = who;
  document.querySelector(".mm-name").textContent = who;
  $("mmSub").textContent = me ? me.email : "Not signed in";
  const initial = me ? (curatorMode ? CURATOR[0] : who[0].toUpperCase()) : "You";
  document.querySelector(".mm-head .meav").textContent = initial;
  document.querySelector("#me .meav").textContent = initial;
  $("signIn").hidden = !!me; $("signOut").hidden = $("setName").hidden = !me;
  // The avatar is where you sign in: say so, and say who you are once you have.
  const tip = me ? `Signed in as ${who} — settings, sign out` : "Sign in or settings";
  $("me").title = tip; $("me").setAttribute("aria-label", tip);
}
} }, "sync curator UI");
