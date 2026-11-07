# v4 request: readers' messages are shared (D1, through account.js), not kept in one browser;
# the live socket refetches them when anyone posts, and "… is typing" is real once in the present.

sub1(qr{const POSTS_KEY = "gc\.posts\.v1";\nlet POSTS = load\(POSTS_KEY, \[\]\); if \(!Array\.isArray\(POSTS\)\) POSTS = \[\];\n},
  sub { q{let POSTS = account.posts;   // everyone's messages, from the server (account.js); p.mine marks yours
account.onChange(() => {
  if (POSTS === account.posts) return;
  POSTS = account.posts; keys.members = null; mineKey = null; renderMine(); refreshSlots(); renderThread();
});
} }, "shared posts");
sub1(qr{const savePosts = \(\) => save\(POSTS_KEY, POSTS\);\n}, "", "no local posts");

# Each message shows its author; only your own can be edited or deleted.
sub1(qr{function myPostHTML\(p\) \{\n.*?\n\}\n}s, sub { q{function myPostHTML(p) {
  const who = p.mine ? "You" : p.authorName || "A listener";
  const initials = p.mine ? "You" : who.split(/\s+/).map(w => w[0]).join("").slice(0, 2).toUpperCase();
  return `<div class="msg mine${p.mine ? "" : " other"}" data-post="${esc(p.id)}"><div class="av">${esc(initials)}</div><div><div class="who">${esc(who)}<span class="when">${esc(fmtNow(p.at))}${p.edited ? " · edited" : ""}</span></div>
    <div class="body plain-text">${esc(p.text)}</div></div>
    ${p.mine ? `<div class="mine-tools"><button data-edit="${esc(p.id)}" title="Edit your message">✎ Edit</button><button data-del="${esc(p.id)}" title="Delete your message">Delete</button></div>` : ""}</div>`;
}
} }, "post markup");
sub1(qr{  \.msg\.mine \.av \{ background: #6b5a44; font-size: 12px; \}\n}, sub { "  .msg.mine .av { background: #6b5a44; font-size: 12px; }\n  .msg.mine.other .av { background: #8a6f4e; }\n" }, "other reader avatar");
sub1(qr{    const p = POSTS\.find\(x => x\.id === id\); if \(!p \|\| m\.classList\.contains\("editing"\)\) return;}, sub { q{    const p = POSTS.find(x => x.id === id); if (!p || !p.mine || m.classList.contains("editing")) return;} }, "edit own only");
sub1(qr{  if \(p && text !== null && text\.trim\(\) && text\.trim\(\) !== p\.text\) \{ p\.text = text\.trim\(\); p\.edited = new Date\(\)\.toISOString\(\); savePosts\(\); \}},
  sub { q{  if (p && p.mine && text !== null && text.trim() && text.trim() !== p.text) { p.text = text.trim(); p.edited = new Date().toISOString(); account.editPost(id, p.text); }} }, "save edit");
sub1(qr{  const key = posts\.map\(p => p\.id\)\.join\(\) \+ "\|" \+ modesKey\(\);}, sub { q{  const key = posts.map(p => p.id + (p.edited || "")).join() + "|" + modesKey();} }, "posts key");
sub1(qr{  POSTS\.push\(\{ id: "p" \+ Date\.now\(\)\.toString\(36\) \+ Math\.random\(\)\.toString\(36\)\.slice\(2, 6\), channel: state\.channel,\n    thread: where === "thread" \? openThreadId : null, text, at: new Date\(\)\.toISOString\(\) \}\);\n  savePosts\(\);},
  sub { q{  account.post(state.channel, where === "thread" ? openThreadId : null, text);   // the page refreshes when it's saved} }, "post to server");
sub1(qr{  if \(d && confirmDelete\(d\)\) \{ POSTS = POSTS\.filter\(p => p\.id !== d\.dataset\.del\); savePosts\(\);}, sub { q~  if (d && confirmDelete(d)) { account.deletePost(d.dataset.del);~ }, "delete on server");
sub1(qr{  const mineHere = isPresent\(\) && POSTS\.some\(p => p\.channel === state\.channel\);}, sub { q{  const mineHere = isPresent() && POSTS.some(p => p.mine && p.channel === state.channel);} }, "you row");
sub1(qr{<div class="nm">Guest listener</div><div class="tags"><span class="pill">\$\{POSTS\.filter\(p => p\.channel === state\.channel\)\.length\}},
  sub { q{<div class="nm">${esc(account.me?.displayName || "You")}</div><div class="tags"><span class="pill">${POSTS.filter(p => p.mine && p.channel === state.channel).length}} }, "you row count");

# Real typing: tell others while you type; show others' typing in the present (not during playback).
sub1(qr{(function startSegment\(\) \{\n)}, sub { q{let liveTypingTimer = 0;
account.onTyping(m => {
  if (!isPresent() || state.playing || m.channel !== state.channel) return;
  const where = m.thread ? " in a thread" : "";
  $("typing").innerHTML = `<span class="dots"><i></i><i></i><i></i></span><span><b>${esc(m.name)}</b> is typing${where}…</span>`;
  clearTimeout(liveTypingTimer);
  liveTypingTimer = setTimeout(() => { if (!state.playing) $("typing").innerHTML = ""; }, 6000);
});
document.addEventListener("input", e => {
  const f = e.target.closest?.(".compose[data-where]"); if (!f || !e.target.value.trim()) return;
  account.typing(state.channel, f.dataset.where === "thread" ? openThreadId : null);
});
} . $1 }, "live typing");
# v4 request: a message reads "From <source>:" and then the utterance as a block quote.
sub1(qr{      <div class="body">\$\{showQuote\(n\)\}</div><div class="cslot">},
  sub { q{      <div class="body">${n.work ? `<div class="from">From <i>${esc(n.work)}</i>${n.ref ? `, ${esc(n.ref)}` : ""}:</div>` : ""}<blockquote class="mq">${showQuote(n)}</blockquote></div><div class="cslot">} }, "from source");
sub1(qr{(  \.msg\.cont \{ padding-top: 1px; \}\n)}, sub { $1 . q{  .msg .from { font-size: 14px; color: var(--ink-2); margin: 1px 0 2px; }
  .msg blockquote.mq { margin: 0; padding: 1px 0 1px 12px; border-left: 3px solid var(--rule); }
} }, "from source css");
# In thread previews a reader's reply shows its author ("You" only for your own).
sub1(qr{const miniAv = r => r\.mine \? `<span class="mini" style="background:#6b5a44">Y</span>`}, sub { q{const miniAv = r => r.mine ? `<span class="mini" style="background:${r.post.mine ? "#6b5a44" : "#8a6f4e"}">${esc((r.post.mine ? "You" : r.post.authorName || "?")[0].toUpperCase())}</span>`} }, "reply avatar");
sub1(qr{  const name = r\.mine \? "You" :}, sub { q{  const name = r.mine ? (r.post.mine ? "You" : r.post.authorName || "A listener") :} }, "reply author");
