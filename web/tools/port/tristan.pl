# v4 request: no "Curator" anywhere on the page. Tristan's replies are just Tristan's comments.
sub1(qr{<button role="tab" data-tab="cur">Curator</button>}, sub { q{<button role="tab" data-tab="cur">Editing</button>} }, "settings tab");
sub1(qr{`<div class="mhead">Curator</div>}, sub { q{`} }, "members heading");
sub1(qr{<div class="fhint">Editable in curator mode \(Settings → Curator\)\.</div>}, sub { q{<div class="fhint">Only ${esc(CURATOR)} can edit this, when signed in.</div>} }, "context hint");
sub1(qr{<div class="meta">Curator of this thread</div>}, sub { q{<div class="meta">Replied in this thread</div>} }, "thread meta");
sub1(qr{download: "curator-edits\.json"}, sub { q{download: "tristan-edits.json"} }, "export file name");
