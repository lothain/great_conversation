# Getting the source text for the remaining entries

50 entries in `data/aphorisms.json` still have open `needs`. Each one's `provenance_notes` says what is
missing. `python tools/sources.py list` writes the current worklist to `sources/WORKLIST.md`.

## How authentication works

You log in to each site **once, by hand**, in a browser that `tools/sources.py` controls. The browser
keeps its cookies in a separate profile folder, `tools/.browser-profile/`. Later runs reuse that profile
headlessly, so every request goes out as you, logged in. No passwords are stored in scripts or files, and
the script never types one. Add `tools/.browser-profile/` and `sources/` to `.gitignore`. The profile
holds live session cookies.

```sh
pip install playwright
python -m playwright install chromium
python tools/sources.py login     # one tab per site: log in, Borrow any IA books (see below), close the window
python tools/sources.py fetch     # all open entries, or: fetch hawking-mind-of-god levi-strauss-lecon-d-ecriture
python tools/sources.py grep      # FOUND / missing for each quote in the saved text
```

`fetch` saves `sources/<id>/0.txt` (or `.pdf`), one file per link, plus a `REPORT.md`. For Internet
Archive books it also runs a search-inside for each quote and records page numbers and snippets.

When a session expires, run `login` again. Two sites don't fit this approach:
- **Cloudflare/Fandom (Memory Alpha):** blocks headless browsers. Open the page by hand if needed.
- **augustinus.it:** returns code 999 to scripts. Open the page by hand if needed.

## What to log in to, by group

### 1. Internet Archive borrow-only books (17 entries)
These are the Lévi-Strauss, Derrida ×2, Foucault, Dick ×2, McLuhan ×2, Heisenberg, Hawking ×2 and
Bloomfield entries.

- **Account:** free archive.org account.
- **Borrowing:** each book has to be on loan, 1 hour or 14 days, at most 10 at once. Borrow them in the
  `login` browser, then run `fetch`.
- **If the search comes back empty:** Internet Archive doesn't always expose search-inside to scripts even
  during a loan. `REPORT.md` will say "search not available" in that case. Open the book in the same
  browser and use the magnifier search with the phrase that `grep` prints.
- **Record what you find:** the page number for `ref`.

### 2. Paywalled journals and publisher pages
- **Ingerman, ACM (CACM 10(3) p. 137):** use an ACM account or an institutional login. Many public
  libraries give remote ACM access with a library card. Check the quote against the scan.
- **Neurath, Springer (Erkenntnis 3, DOI 10.1007/BF01886420):** institutional login.
- **Einstein "Physik und Realität" and Naur "Programming as Theory Building", ScienceDirect:** institutional
  login. The Naur PDF is also linked free from several course pages, but it is a scan, so read it by eye.
- **Quine and other JSTOR items:** a free JSTOR account allows 100 articles a month online.

### 3. Open Library catalog links (8 entries: Tolkien ×4, Lewis ×4, Quine *Word and Object*, Ficker letter)
The links only need to load. Run `fetch` and check that `REPORT.md` shows HTTP 200. Tolkien and Lewis
have no legal free text. To check those quotes, borrow the book from Open Library or a library and edit
the entry by hand.

### 4. Old scans whose text can't be searched (11 `verify_orig` entries)
These are Diels–Kranz, Nāgeśa, the Metres of Boethius and Humboldt. They are public domain and need no
login. The problem is the OCR: it can't handle polytonic Greek, Devanagari, Old English or old German
type. Open the Internet Archive reader at the page listed in `ref` and compare by eye. For Diels–Kranz,
the 1912 3rd edition is the item linked; fragment numbers are the same as in later editions.

### 5. No online text at all
- **Lichtenberg, Sudelbücher E 215 and H 146; Hofmannsthal, *Ein Brief*:** try the Zeno.org pages again
  (`http://www.zeno.org/Literatur/M/Lichtenberg,+Georg+Christoph/Aphorismen`, and the same path for
  Hofmannsthal). Zeno.org was erroring when checked. Otherwise use the Deutsches Textarchiv
  (deutschestextarchiv.de).
- **Aquinas "straw":** *Fontes vitae S. Thomae Aquinatis*, a library item.
- **Mallarmé to Degas:** Valéry, *Degas Danse Dessin*. Gallica may have the 1936 Vollard edition;
  check gallica.bnf.fr (free account).
- **Sirafi–Mattā debate:** al-Tawḥīdī, *al-Imtāʿ wa-l-muʾānasa*. The Arabic text is on archive.org
  (search "الإمتاع والمؤانسة"). Margoliouth's 1905 English translation (JRAS) is on JSTOR.
- **Foucault, *L'Ordre du discours*:** the Gallimard catalog page is enough to clear `source_url`.
- **Oppenheimer–Groves letter, 20 Oct 1962:** the Library of Congress Oppenheimer Papers finding aid.
  The letter is quoted in Rhodes, *The Making of the Atomic Bomb* (1986), which is on Internet Archive
  (group 1 method; record the page you find).

## Recording results
For each entry you settle, edit `data/aphorisms.json`:
- remove the item from `needs`;
- correct `quotes[].orig` and `ref` (page) if they differ from the source;
- replace the "not checked" sentence in `provenance_notes` with what you checked against, e.g.
  "Checked against Bantam 1988, p. 175."

Then run `cargo run -p aph-data -- check` and `python tools/sources.py list` to confirm the entry has
left the worklist.
