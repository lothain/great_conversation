# Port v3's page onto v4's Bevy globe. Every pattern must match exactly once.
use File::Basename; use File::Spec; my $dir = File::Spec->rel2abs(dirname(__FILE__));
local $/;
my ($src, $blockfile, $out) = @ARGV;
open my $in, "<", $src or die; my $s = <$in>;
open my $b, "<", $blockfile or die; my $block = <$b>;
sub sub1 { my ($re, $rep, $name) = @_; my $n = 0; $n++ while $s =~ /$re/g; die "pattern '$name' matched $n times\n" unless $n == 1; if (ref $rep) { $s =~ s/$re/$rep->()/e } else { $s =~ s/$re/$rep/ } }
sub suball { my ($re, $rep, $name) = @_; my $n = 0; $n++ while $s =~ /$re/g; die "pattern '$name' matched none
" unless $n; $s =~ s/$re/$rep->()/ge; }
# head: no globe.gl, no data.js, no CDN fonts; Trunk builds the WASM and copies data and fonts
sub1(qr{<link rel="preconnect" href="https://fonts\.googleapis\.com">\n<link href="https://fonts\.googleapis\.com/css2\?family=EB\+Garamond[^"]*" rel="stylesheet">\n}, qq{<link rel="stylesheet" href="fonts/eb-garamond.css">\n}, "google fonts");
sub1(qr{<script src="https://cdn\.jsdelivr\.net/npm/globe\.gl[^\n]*\n<script src="https://cdn\.jsdelivr\.net/npm/topojson[^\n]*\n<script src="data\.js"></script>\n}, qq{<link data-trunk rel="rust" href="../crates/aph-globe" data-bin="aph-globe" data-bindgen-target="web" data-wasm-opt="z" data-wasm-opt-params="--enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-sign-ext --enable-reference-types --enable-multivalue --enable-mutable-globals" />\n<link data-trunk rel="copy-file" href="globe.js" />\n<link data-trunk rel="copy-file" href="account.js" />\n<link data-trunk rel="copy-dir" href="../data" />\n<link data-trunk rel="copy-dir" href="portraits" />\n<link data-trunk rel="copy-dir" href="fonts" />\n}, "head");
$s =~ s{https://cdn\.jsdelivr\.net/npm/dseg\@0\.46\.0/fonts/DSEG(7|14)-Classic/}{fonts/}g;
sub1(qr{<style>\n}, "<style>\n  #globeCanvas { display: block; width: 100%; height: 100%; outline: none; }\n", "style");
sub1(qr{<div id="globe"></div>}, q{<div id="globe"><canvas id="globeCanvas"></canvas></div>}, "canvas");
sub1(qr{<script>\nconst D = window\.APH;\n}, qq{<script type="module">\nimport { boot } from "./globe.js";\nconst { D, globe } = await boot("#globeCanvas");\n}, "boot");
sub1(qr{const RELCOL = \{\n.*?\n\};\n}s, "const RELCOL = D.colors.religion;\n", "relcol");
sub1(qr{const civColor = k => CIVCOL\[civKeys\.indexOf\(k\) % CIVCOL\.length\];}, q{const civColor = k => D.civs.find(c => c.civ === k)?.color || "#9c8f78";}, "civcolor");
sub1(qr{// ---------- parchment texture \+ legend ----------\n.*?(?=// ---------- channel list ----------)}s, $block, "globe block");
sub1(qr{  const zb = zoomBucket\(\), nodeKey = mk \+ "\|" \+ zb \+ "\|" \+ (\w+)\.map\(n => n\.id\)\.join\(\);\n  if \(nodeKey !== keys\.nodes\) \{\n.*?\n    (renderFeed\([^\n]*)\n  \}\n}s, sub { "  globe.setTime(y, w);\n  const nodeKey = mk + \"|\" + $1.map(n => n.id).join();\n  if (nodeKey !== keys.nodes) { keys.nodes = nodeKey; $2 }\n" }, "render nodes");
sub1(qr{  const ids = new Set\(nodes\.map\(n => n\.workId\)\.filter\(Boolean\)\).*?globe\.arcsData\([^\n]*\n}s, "  // Threads and \@mentions are drawn and timed on the GPU.\n", "arcs");
sub1(qr{    const polys = \[\], relMask = relSet\.size > 0;\n.*?    paintCivs\(polys\);\n}s, "    globe.civMask(civSet);\n    globe.religionMask(relSet);\n", "civ paint");
sub1(qr{  document\.querySelectorAll\("\.gdot\.hl"\)\.forEach\(x => x\.classList\.remove\("hl"\)\);\n  if \(m\) clusters\.find\(c => c\.ids\.includes\(m\.dataset\.id\)\)\?\.dot\.classList\.add\("hl"\);\n\}\);\n}, "  globe.highlight(m ? m.dataset.id : null);\n});\nfeed.addEventListener(\"mouseleave\", () => globe.highlight(null));\n", "feed hover");
# v4 request: in the utterances tab the original always comes first, both as block quotes
sub1(qr{  const origFirst = n\.trans && modeFor\(n\.lang\) === "orig";\n  const quote = n\.orig \? \(origFirst \? [^\n]*\n[^\n]*\n}, "  // Always the original first, then the English when there is a translation; both block quotes.\n  const quote = n.orig ? `<blockquote class=\"orig\" lang=\"\${esc(n.lang)}\">\${esc(n.orig)}</blockquote>`\n                         + (n.trans ? `<blockquote class=\"en\" lang=\"en\">\${gloss(n.trans)}</blockquote>` : \"\")\n", "utterance quote");
sub1(qr{  \.udetail \.uo \{[^\n]*\n}, "  /* Quotes: the original always first, the English below it, each as a block quote. */\n  .udetail blockquote { margin: 0 0 8px; padding: 6px 0 6px 14px; border-left: 3px solid var(--rule); font-size: 18px; line-height: 1.4; }\n  .udetail blockquote.orig { color: var(--ink); }\n  .udetail blockquote.en { font-size: 16.5px; color: var(--ink-2); border-left-style: dotted; }\n", "quote css");
# v4 request: the profile panel has no "Follow the conversation" toggle and no diamond anchor.
sub1(qr{<label class="follow"><input type="checkbox" id="followChk"> Follow the conversation</label>\n    }, "", "follow markup");
sub1(qr{  #profHead \.follow \{[^\n]*\n}, "  #profHead { justify-content: flex-end; }\n", "follow css");
sub1(qr{  <i id="profArrow"></i>\n}, "", "arrow markup");
sub1(qr{  #profArrow \{[^\n]*\n}, "", "arrow css");
sub1(qr{ \$\("followChk"\)\.checked = false;}, "", "follow reset");
sub1(qr{\$\("followChk"\)\.onchange = [^\n]*\n}, "", "follow handler");
sub1(qr{  el\.style\.setProperty\("--arrow-y"[^\n]*\n  \$\("profArrow"\)\.style\.display = [^\n]*\n}, "", "arrow placement");
# v4 request: no slashes in the clock's date. "!" is DSEG's blank, digit-wide cell, so the gaps
# line up with the unlit "ghost" segments behind the digits.
sub1(qr{(cell d7 date"><i class="g">)8888/88/88}, sub { "${1}8888!88!88" }, "clock date ghost");
sub1(qr{\$\("ckDate"\)\.textContent = `\$\{String\(disp\)\.padStart\(4, "0"\)\}/\$\{p2\(t\.mo\)\}/\$\{p2\(t\.day\)\}`;}, q{$("ckDate").textContent = `${String(disp).padStart(4, "0")}!${p2(t.mo)}!${p2(t.day)}`;}, "clock date text");
# The globe reports its own clicks (including empty ones), so the page's click-away must not
# also close the profile on them: that closed and reopened it on every dot click.
sub1(qr{e\.target\.closest\("#prof, #members, }, q{e.target.closest("#prof, #members, #globe, }, "globe click-away");
# v4 request: the channel list's first section is called "Topics".
sub1(qr{sect\("Text channels", "all"\)}, q{sect("Topics", "all")}, "topics heading");
sub1(qr{k === "all" \? "Text channels"}, q{k === "all" ? "Topics"}, "topics optgroup");
do "$dir/speak.pl" or die "speak.pl: $@";
do "$dir/auth.pl" or die "auth.pl: $@";
do "$dir/welcome.pl" or die "welcome.pl: $@";
do "$dir/popout.pl" or die "popout.pl: $@";
do "$dir/live.pl" or die "live.pl: $@";
do "$dir/phone.pl" or die "phone.pl: $@";
do "$dir/tristan.pl" or die "tristan.pl: $@";
# v4 request: no top bar; just a floating circular avatar in the top right.
sub1(qr{<header id="topbar">\n  <h1>The Great Conversation</h1>\n  <span style="flex:1"></span>\n  (<button id="me" [^\n]*</button>)\n</header>\n}, sub { "$1\n" }, "top bar markup");
sub1(qr{  #app \{ position: absolute; top: 56px;}, "  #app { position: absolute; top: 0;", "app top");
sub1(qr{  #me \{ [^\n]*\n}, "  /* The only thing left of the top bar: a floating, circular avatar in the top right. */\n  #me { position: fixed; top: 10px; right: 12px; z-index: 46; width: 40px; height: 40px; padding: 0; border-radius: 50%; display: grid; place-items: center;\n    background: rgba(42,37,32,.85); border: 1px solid rgba(239,228,200,.35); box-shadow: 0 4px 14px rgba(0,0,0,.35); cursor: pointer; }\n  #me:hover, #me[aria-expanded=\"true\"] { border-color: rgba(239,228,200,.7); }\n  #me .mename, #me .caret { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }\n", "me button css");
open my $o, ">", $out or die; print $o $s; print "PORTED\n";
