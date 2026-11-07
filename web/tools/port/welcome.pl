# v4 request: the welcome's Begin button is centred, with a "Don't show this message again" box.
sub1(qr{    <button id="wgo">Begin</button>\n}, sub { qq{    <div class="wfoot"><button id="wgo">Begin</button>\n      <label class="wskip"><input type="checkbox" id="wskip"> Don't show this message again</label></div>\n} }, "welcome footer");
sub1(qr{  #wgo \{[^\n]*\n}, sub { q{  #welcome .wfoot { display: flex; flex-direction: column; align-items: center; gap: 10px; margin-top: 12px; }
  #wgo { font: inherit; font-size: 16px; background: var(--ink); color: var(--panel); border: 0; border-radius: 6px; padding: 7px 26px; cursor: pointer; }
  #welcome .wskip { display: flex; align-items: center; gap: 6px; font-size: 14px; color: var(--ink-2); cursor: pointer; }
} }, "welcome css");
sub1(qr{const closeWelcome = \(\) => \{ welcome\.hidden = true; save\("gc\.welcomed", true\); \};\n}, sub { q{// Shown on every visit until "Don't show this message again" is ticked.
$("wskip").checked = !!load("gc.welcomed", false);
const closeWelcome = () => { welcome.hidden = true; save("gc.welcomed", $("wskip").checked); autoplay(); };
} }, "welcome close");
sub1(qr{\$\("openWelcome"\)\.onclick = \(\) => \{ meMenu\.hidden = true; \$\("welcome"\)\.hidden = false; \};}, sub { q{$("openWelcome").onclick = () => { meMenu.hidden = true; $("wskip").checked = !!load("gc.welcomed", false); $("welcome").hidden = false; };} }, "how this works");
# v4 request: the conversation starts playing on its own, once per visit: when the welcome is
# dismissed, or straight away when it's been switched off.
sub1(qr{if \(!load\("gc\.welcomed", false\)\) welcome\.hidden = false;\n}, sub { q{let autoplayed = false;
const autoplay = () => { if (autoplayed) return; autoplayed = true; if (!state.playing) setTimeout(() => $("play").click(), 0); };
if (!load("gc.welcomed", false)) welcome.hidden = false; else autoplay();
} }, "autoplay on load");
# v4 request: the clock reads date, era, then time (the time belongs to the right of the day).
sub1(qr{<div class="lcd">(<span class="cell d7"><i class="g">88:88</i>.*?</b></span>)(<span class="cell d7 date">.*?</b></span>)(<span class="cell m14">.*?</b></span>)</div>}, sub { "<div class=\"lcd\">$2$3$1</div>" }, "clock order");
# v4: our favicon.
sub1(qr{<link rel="stylesheet" href="fonts/eb-garamond.css">\n}, sub { qq{<link rel="stylesheet" href="fonts/eb-garamond.css">\n<link rel="icon" type="image/svg+xml" href="favicon.svg">\n<link data-trunk rel="copy-file" href="favicon.svg" />\n} }, "favicon");
# The curator's own welcome wording (recovered from deployment 6a9adbd9).
sub1(qr{(<h1 id="wtitle">The Great Conversation</h1>\n)    <div class="sub">#language-meaning-world</div>\n.*?</ul>\n}s, sub { $1 . q{    <p>Human ideas have evolved over time and by reacting to one another.</p>
    <ul>
      <li><b>The Great Conversation</b> is meant to help explore this process</li>
      <li><b>Time</b> is set with the slider along the bottom.</li>
      <li><b>Members</b> represent humans or books (written by humans) who have contributed.</li>
      <li><b>Utterances</b> represent an encapsulated idea or contribution</li>
    </ul>
} }, "welcome wording");
