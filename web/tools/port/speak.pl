# v4 request: "read aloud" is a circular brown speaker button right of the original quote.
sub1(qr{    <div class="hear" [^\n]*\n}, "", "hear footer");
sub1(qr{  \.hear \{[^\n]*\n  \.hear button \{[^\n]*\n}, sub { q{  /* Read aloud: a circular speaker button to the right of the original. */
  .udetail .qrow { display: flex; align-items: flex-start; gap: 10px; }
  .udetail .qrow blockquote { flex: 1; min-width: 0; }
  .udetail .speak { flex: none; width: 34px; height: 34px; margin-top: 6px; padding: 0; border: 0; border-radius: 50%;
    display: grid; place-items: center; background: #7a5a3a; color: #f4ead2; box-shadow: 0 1px 3px rgba(60,40,20,.35); cursor: pointer; }
  .udetail .speak svg { width: 18px; height: 18px; }
  .udetail .speak:hover:not([aria-disabled="true"]) { background: #8a4a2a; }
  /* Not available yet: looks off, but still shows its tooltip (a truly disabled button wouldn't). */
  .udetail .speak[aria-disabled="true"] { background: #b9a88c; color: #f4ead2; box-shadow: none; opacity: .6; cursor: not-allowed; }
} }, "hear css");
sub1(qr{  const quote = n\.orig \? `<blockquote class="orig" lang="\$\{esc\(n\.lang\)\}">\$\{esc\(n\.orig\)\}</blockquote>`\n}, sub { q{  const quote = n.orig ? `<div class="qrow"><blockquote class="orig" lang="${esc(n.lang)}">${esc(n.orig)}</blockquote>${speakButton(n)}</div>`
} }, "quote with speaker");
sub1(qr{(function utteranceDetail\(n\) \{\n)}, sub { q{// Read aloud (coming later): each utterance in its own language by a language-specific voice.
const speakButton = n => `<button type="button" class="speak" aria-disabled="true" tabindex="0" aria-label="Hear it read aloud (coming soon)" title="Hear it read aloud in ${esc(langLabel(n.lang))} (coming soon)"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9.5v5h3.5L12 18.5v-13L7.5 9.5z" fill="currentColor"/><path d="M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg></button>`;
} . $1 }, "speaker button");
