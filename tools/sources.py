"""Fetch source text for aphorism entries that still have open `needs`.

Uses a persistent Chromium profile (tools/.browser-profile, git-ignore it) so that you log in
to each site once, by hand, and every later scripted run reuses those cookies. Nothing here
stores or types passwords.

    pip install playwright && python -m playwright install chromium

    python tools/sources.py list                # worklist -> sources/WORKLIST.md
    python tools/sources.py login               # opens a browser; log in to each site, then close it
    python tools/sources.py fetch [ID ...]      # saves page text/PDFs to sources/<id>/
    python tools/sources.py grep                # looks for each quote in what was fetched

`fetch` also runs a search-inside on Internet Archive books using your logged-in session. For
borrow-only books you must have the book on loan (click "Borrow" in the login browser first).
"""
import json, re, sys, unicodedata
from pathlib import Path
from urllib.parse import quote

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data" / "aphorisms.json"
OUT = ROOT / "sources"
PROFILE = Path(__file__).resolve().parent / ".browser-profile"

LOGIN_SITES = [
    ("Internet Archive (borrow-only books)", "https://archive.org/account/login"),
    ("ACM Digital Library (institutional or ACM login)", "https://dl.acm.org/action/showLogin"),
    ("JSTOR", "https://www.jstor.org/action/showLogin"),
    ("SpringerLink (Neurath, Einstein papers)", "https://link.springer.com/signup-login"),
    ("ScienceDirect (Naur, Einstein Franklin Inst.)", "https://www.sciencedirect.com/user/login"),
    ("Open Library (same login as Internet Archive)", "https://openlibrary.org/account/login"),
]


def entries():
    d = json.loads(DATA.read_text(encoding="utf-8"))
    return [e for e in d["entries"] if e.get("needs")]


def norm(s):
    s = unicodedata.normalize("NFKD", s)
    s = "".join(c for c in s if not unicodedata.combining(c))
    return re.sub(r"[^\w]+", " ", s.lower()).strip()


def probe(q):
    """A distinctive 4-6 word run from the quote, for searching."""
    words = norm(q).split()
    if len(words) <= 6:
        return " ".join(words)
    best = max(range(len(words) - 5), key=lambda i: sum(len(w) for w in words[i:i + 6]))
    return " ".join(words[best:best + 6])


def cmd_list():
    OUT.mkdir(exist_ok=True)
    lines = ["# Sourcing worklist", "", "| id | needs | source | what to check |", "|---|---|---|---|"]
    for e in entries():
        u = e["source"].get("url") or "—"
        qs = "; ".join(q["orig"][:80] for q in e["quotes"]) or "(no quote)"
        lines.append(f"| `{e['id']}` | {', '.join(e['needs'])} | {u} | {qs.replace('|', '/')} |")
    (OUT / "WORKLIST.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{len(lines) - 4} entries -> {OUT / 'WORKLIST.md'}")


def browser(p, headless):
    return p.chromium.launch_persistent_context(str(PROFILE), headless=headless, accept_downloads=True)


def cmd_login():
    from playwright.sync_api import sync_playwright
    with sync_playwright() as p:
        ctx = browser(p, headless=False)
        for name, url in LOGIN_SITES:
            pg = ctx.new_page()
            pg.goto(url)
            print("opened:", name)
        print("Log in on each tab (and Borrow any IA books you want searched). Close the window when done.")
        ctx.pages[0].wait_for_event("close", timeout=0)
        ctx.close()


def ia_search(ctx, ident, phrase):
    """Search inside an IA book using the logged-in session. Returns list of (page, snippet)."""
    meta = ctx.request.get(f"https://archive.org/metadata/{ident}").json()
    server, d = meta.get("server"), meta.get("dir")
    if not server:
        return [("?", "metadata unavailable (not logged in, or item restricted)")]
    url = (f"https://{server}/fulltext/inside.php?item_id={ident}&doc={ident}"
           f"&path={quote(d)}&q={quote(phrase)}")
    r = ctx.request.get(url)
    try:
        j = r.json()
    except Exception:
        return [("?", f"HTTP {r.status}: search not available — borrow the book and use the in-reader search")]
    if j.get("error"):
        return [("?", j["error"])]
    return [(m.get("par", [{}])[0].get("page", "?"), re.sub(r"\{\{\{|\}\}\}", "**", m.get("text", "")))
            for m in j.get("matches", [])]


def cmd_fetch(ids):
    from playwright.sync_api import sync_playwright
    todo = [e for e in entries() if not ids or e["id"] in ids]
    with sync_playwright() as p:
        ctx = browser(p, headless=True)
        for e in todo:
            dest = OUT / e["id"]
            dest.mkdir(parents=True, exist_ok=True)
            report = [f"# {e['id']}", f"needs: {e['needs']}", ""]
            urls = [e["source"].get("url")] + [a["url"] for a in e.get("see_also", [])]
            for i, u in enumerate(x for x in urls if x):
                try:
                    pg = ctx.new_page()
                    resp = pg.goto(u, timeout=60000, wait_until="domcontentloaded")
                    pg.wait_for_timeout(2500)  # let JS-rendered text (e.g. nietzschesource) appear
                    ctype = (resp.headers.get("content-type", "") if resp else "")
                    if "pdf" in ctype:
                        (dest / f"{i}.pdf").write_bytes(resp.body())
                        report.append(f"- {u}: saved {i}.pdf")
                    else:
                        (dest / f"{i}.txt").write_text(pg.inner_text("body"), encoding="utf-8")
                        report.append(f"- {u}: HTTP {resp.status if resp else '?'}, saved {i}.txt")
                    pg.close()
                except Exception as ex:
                    report.append(f"- {u}: FAILED {ex.__class__.__name__}: {str(ex)[:120]}")
                m = re.match(r"https://archive\.org/details/([^/?#]+)", u)
                if m:
                    for q in e["quotes"]:
                        ph = probe(q["orig"])
                        report.append(f"  - IA search '{ph}':")
                        for page, snip in ia_search(ctx, m.group(1), ph)[:5]:
                            report.append(f"    - p.{page}: {snip[:300]}")
            (dest / "REPORT.md").write_text("\n".join(report) + "\n", encoding="utf-8")
            print("\n".join(report), "\n")
        ctx.close()


def cmd_grep():
    for e in entries():
        dest = OUT / e["id"]
        texts = [norm(t.read_text(encoding="utf-8", errors="ignore")) for t in dest.glob("*.txt")] if dest.exists() else []
        for q in e["quotes"]:
            ph = probe(q["orig"])
            hit = any(ph in t for t in texts)
            print(f"{'FOUND ' if hit else 'missing'} {e['id']}: {ph}")


if __name__ == "__main__":
    c, *rest = sys.argv[1:] or ["list"]
    {"list": lambda: cmd_list(), "login": lambda: cmd_login(), "fetch": lambda: cmd_fetch(rest),
     "grep": lambda: cmd_grep()}[c]()
