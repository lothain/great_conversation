# v4 request: an acceptable phone experience. Checked with APH_PHONE=1 node tools/inspect.mjs.
#   - the welcome card fits the screen; the bottom bar fits (no overlapping year labels, clock visible)
#   - the globe is behind a World button on narrow screens (and only downloads when opened)
#   - the globe takes touch (drag, pinch, tap) without scrolling or zooming the page
#   - touch targets are at least ~36px; the channel menu isn't under the floating avatar

my $globeIcon = q{<svg width="20" height="20" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.6"><circle cx="12" cy="12" r="9"/><ellipse cx="12" cy="12" rx="4" ry="9"/><path d="M3 12h18M5 7h14M5 17h14"/></svg>};
sub1(qr{(      <button id="chanMenuBtn" )}, sub { qq{      <button id="worldBtn" aria-label="Show the globe" title="The World">$globeIcon</button>\n$1} }, "world button");
sub1(qr{(<div class="paneBar"><span>The World</span>)(<button id="popBtn")}, sub { qq{$1<button id="worldClose" aria-label="Back to the conversation" title="Back to the conversation">✕</button>$2} }, "world close");
sub1(qr{(globe\.on\("docked", \(\) => popOut\(false\)\);\n)}, sub { $1 . q{// Phones: the globe is a full-screen view behind the World button.
const setWorld = on => document.body.classList.toggle("world", on);
$("worldBtn").onclick = () => setWorld(!document.body.classList.contains("world"));
$("worldClose").onclick = () => setWorld(false);
} }, "world toggle");
sub1(qr{(  \@media \(max-width: 900px\) \{\n)}, sub { q{  #worldBtn, #worldClose { display: none; }
  #globeCanvas { touch-action: none; }
} . $1 . q{    #worldBtn { display: grid; place-items: center; flex: none; width: 40px; height: 40px; padding: 0; background: none; border: 0; color: inherit; cursor: pointer; }
    #popBtn { display: none; }
    #chanHead { padding-right: 60px; }
    body.world #left { display: flex; position: absolute; inset: 0 0 46px 0; z-index: 25; width: auto; min-width: 0; }
    body.world #server, body.world #chanList, body.world #hsplitL { display: none; }
    body.world #globePane { flex: 1; height: auto; }
    body.world #worldClose { display: grid; width: 36px; height: 28px; }
    body.world #globePane .paneBar { padding-right: 64px; }   /* keep the close button clear of the avatar */
} }, "phone world css");
sub1(qr{(\n</style>\n</head>)}, sub { q{
  @media (max-width: 600px) {
    .wcard { width: 100%; box-sizing: border-box; max-height: calc(100dvh - 32px); overflow-y: auto; padding: 18px 18px 16px; }
    .wcard h1 { font-size: 24px; }
    #ticks { display: none; }
    #scrub { min-width: 60px; }
    #bottombar { gap: 8px; padding-left: 6px; }
    #bottombar #clock { padding: 0 8px; }
    .lcd { gap: 5px !important; } .d7 { font-size: 13px; } .m14 { font-size: 9px; }
    #transport button { min-width: 36px; min-height: 36px; }
    #chanMenuBtn { min-width: 36px; min-height: 36px; }
    #welcome .wskip input { width: 20px; height: 20px; }
  }} . $1 }, "phone css");
