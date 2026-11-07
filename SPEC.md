# aphorisms.json — schema

Top level:

```json
{
  "title": "The Word About Words",
  "subtitle": "Aphorisms on language from the Great Conversation",
  "date": "2026-11-07",
  "author": "@Tristan",
  "sections": [ { "id": "hebrew-bible", "name": "Hebrew Bible", "intro": "..." } ],
  "authors":  { "<author-id>": Author },
  "entries":  [ Entry ]
}
```

## Entry (REVISED — this replaces the earlier Entry and gloss sections)

```json
{
  "id": "darmok",
  "section": "star-trek",
  "author": "star-trek",
  "work": "Star Trek: The Next Generation, \"Darmok\"",
  "ref": "S5E2",
  "date": "1991-09-30",
  "date_note": null,
  "place": { "name": "Paramount lot, Hollywood", "lat": 34.084, "lon": -118.321, "conf": "high", "why": "TNG filmed on Paramount stages" },
  "context": "The Tamarians speak only through allusions to their own myths; Picard is stranded with their captain on El-Adrel[hostile planet].",
  "quotes": [
    { "lang": "en", "orig": "Darmok and Jalad at Tanagra.", "translit": null, "trans": null, "trans_by": null }
  ],
  "interpretation": "Translating every word still leaves you understanding nothing: meaning lives in shared stories.",
  "source": { "label": "Paramount+", "url": null },
  "see_also": []
}
```

The ten things every entry carries: **context, original, transliteration, translation, interpretation, time, position, work, author, source link.**

- `context`: what is happening when the words are said or written ("Gandalf stands before the Doors of Durin", "Syme explains the shrinking dictionary"). 1–2 sentences. Built from the source document's note.
- `quotes[]`: `orig` (exact, as in the document), `translit` (non-Latin scripts only, else null), `trans` (English; null if orig is English), `trans_by` (named translator or null), `lang` (BCP-47).
- `interpretation`: the essence — one or two sentences that highlight why this matters for the theme of language. Built from the rest of the note (the analytic part). Keep the document's own wording where it already does this.
- Every sentence of the old note must land in `context` or `interpretation` (minor trimming of publisher boilerplate allowed — that goes to `source.label`).
- `date` = **time** (EDTF, as before). `place` = **position** (as before).
- `source`: the primary-text link `{label, url}`; `url` null when none confirmed. Publisher/edition info goes in `label`. Any additional links go in `see_also` as `{label, url}`.
- `provenance_notes` (optional string): how the date, attribution, wording and source were settled: judgement calls, disputed or second-hand attributions, secondary sources relied on, and what is still unchecked. Write it so a reader can tell which claims were verified against a text and which were not.
- No `gloss`, no `note` fields.

### Inline glosses

Unfamiliar words are glossed inline, immediately followed by a bracket of **at most 3 words**, inside `context`, `trans`, or `interpretation` (never inside `orig`):

```
word[≤3 word explanation]
```

Examples: `akṣara[letter; imperishable]`, `logos[word, reason]`, `pharmakon[remedy and poison]`, `mellon[Elvish: friend]`, `usus[usage]`, `mora[syllable length unit]`.
Gloss each unfamiliar word once per entry, at first use. Skip words a general reader knows.

## Author

```json
"panini": {
  "name": "Pāṇini",
  "life": "-0499/-0300",
  "places": [
    { "when": "-0399/-0300", "name": "Śalātura, near Gandhāra", "lat": 34.05, "lon": 72.33, "conf": "low" }
  ]
}
```

- `life`: EDTF interval (use `..` for unknown end, e.g. collective works `-0999/..` is not allowed — give the composition span instead).
- `places`: chronological residence timeline, as many stages as are known (birth, study, exile, emigration, final years). Each `when` is an EDTF interval. Every entry's `place` should fall inside one stage of its author's timeline.
