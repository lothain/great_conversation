# Accounts needed for SOURCING.md

No passwords go in files. Log in once in the `python tools/sources.py login` browser (on a machine
with a display), then copy `tools/.browser-profile/` to the server. Keep it private: it holds live cookies.

| # | Account | Cost | Needed for |
|---|---|---|---|
| 1 | archive.org (also used for Open Library) | Free | Group 1: 17 borrow-only books; Rhodes (Oppenheimer letter); Tolkien/Lewis borrows |
| 2 | Institutional login (university/employer library proxy or SSO) | Through your institution | Springer (Neurath), ScienceDirect (Einstein, Naur) |
| 3 | ACM Digital Library (own account, institution, or public library card) | Free with library card | Ingerman, CACM 10(3) |
| 4 | JSTOR | Free, 100 articles/month | Quine etc.; Margoliouth 1905 JRAS |
| 5 | Gallica (BnF) | Free | Valéry, *Degas Danse Dessin* |

No login needed: group 3 link checks, group 4 public-domain scans, Zeno.org, Deutsches Textarchiv,
Library of Congress finding aid, Gallimard catalog.

## Checklist
- [ ] archive.org account (do group 1 in two rounds: max 10 loans at once)
- [ ] Public library card with remote ACM/JSTOR access, or institutional login
- [ ] JSTOR account
- [ ] Gallica account
- [ ] Run `login`, borrow books, run `fetch` soon after (loans: 1 h or 14 days)
