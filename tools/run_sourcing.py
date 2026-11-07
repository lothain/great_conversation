"""Fetch sources, have the Qwen model check each open entry against what was saved, apply
safe patches to data/aphorisms.json, and list everything left over.

    python tools/run_sourcing.py                      # fetch + check + apply
    python tools/run_sourcing.py --dry-run            # propose only, don't touch the data
    python tools/run_sourcing.py --skip-fetch         # reuse what's already in sources/
    python tools/run_sourcing.py --only ID [ID ...]   # just these entries
    python tools/run_sourcing.py --model-host http://plex-hq-7f72-1:11434

Safety:
  - data/aphorisms.json is backed up to data/backups/ before any change.
  - A model's claim is only applied if the quoted evidence (and any corrected wording) actually
    appears in the saved source text. Otherwise the entry goes to the leftovers list.
  - After each change `cargo run -p aph-data -- check` runs; on failure that change is reverted.

Outputs: sources/CHANGES.md (before/after per entry), sources/LEFTOVERS.md (what still needs you).
"""
import argparse, json, re, shutil, subprocess, sys, time, unicodedata, urllib.request
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data" / "aphorisms.json"
OUT = ROOT / "sources"
MODEL = "qwen3.6:35b"
MAX_CHARS = 24000  # source text sent per entry (fits a 32k-token context with room to spare)

ap = argparse.ArgumentParser()
ap.add_argument("--model-host", default="http://plex-hq-7f72-1:11434")
ap.add_argument("--dry-run", action="store_true")
ap.add_argument("--skip-fetch", action="store_true")
ap.add_argument("--only", nargs="*", default=[])
args = ap.parse_args()


def norm(s):
    s = unicodedata.normalize("NFKD", s or "")
    s = "".join(c for c in s if not unicodedata.combining(c))
    return re.sub(r"[^\w]+", " ", s.lower()).strip()


def load():
    return json.loads(DATA.read_text(encoding="utf-8"))


def save(d):
    DATA.write_text(json.dumps(d, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")


def check():
    r = subprocess.run(["cargo", "run", "-q", "-p", "aph-data", "--", "check"], cwd=ROOT,
                       capture_output=True, text=True)
    return r.returncode == 0, (r.stdout + r.stderr)[-1500:]


def source_text(eid):
    """All saved text for an entry: page text, PDF text if pypdf is installed, and REPORT.md."""
    dest = OUT / eid
    if not dest.exists():
        return "", ""
    parts = [t.read_text(encoding="utf-8", errors="ignore") for t in sorted(dest.glob("*.txt"))]
    for pdf in sorted(dest.glob("*.pdf")):
        try:
            from pypdf import PdfReader
            parts.append("\n".join((p.extract_text() or "") for p in PdfReader(pdf).pages))
        except Exception as ex:
            parts.append(f"[PDF {pdf.name} not readable: {ex.__class__.__name__}]")
    report = (dest / "REPORT.md").read_text(encoding="utf-8", errors="ignore") if (dest / "REPORT.md").exists() else ""
    return "\n\n---\n\n".join(parts), report


def window(text, entry):
    """Trim long text to the parts around the quote's words, so the model sees the relevant bit."""
    if len(text) <= MAX_CHARS:
        return text
    words = {w for q in entry["quotes"] for w in norm(q.get("orig")).split() if len(w) > 5}
    if not words:
        return text[:MAX_CHARS]
    chunks = [text[i:i + 2000] for i in range(0, len(text), 2000)]
    ranked = sorted(range(len(chunks)), key=lambda i: -sum(w in norm(chunks[i]) for w in words))
    keep = sorted(ranked[: MAX_CHARS // 2000])
    return "\n[...]\n".join(chunks[i] for i in keep)


PROMPT = """You check one entry of a scholarly aphorism dataset against source text that was saved from the web.

Entry (JSON):
{entry}

Open needs: {needs}
  - verify_quote: confirm each quote's "orig" wording (and "ref" page/section if visible) against the source.
  - verify_orig: confirm the original-language wording.
  - source_url: confirm the saved page really is the cited work (not a login page, error, or a different work).

Fetch report:
{report}

Saved source text:
<<<
{text}
>>>

Rules:
- Use ONLY the saved source text. Never rely on memory.
- "evidence" must be copied character-for-character from the source text.
- If the source is a login page, error page, OCR garbage, or doesn't contain the passage, use status "unresolved" and say why.

Reply with JSON only:
{{"status": "verified" | "corrected" | "unresolved",
  "resolved_needs": ["..."],
  "quote_fixes": [{{"index": 0, "orig": "corrected wording copied from the source"}}],
  "ref": "page or section if the source shows it, else null",
  "evidence": "exact excerpt from the source text",
  "provenance_note": "one sentence for provenance_notes, e.g. 'Checked against <edition>, p. N.'",
  "reason": "if unresolved: why, and what a human should do"}}"""


def ask(entry, report, text):
    body = json.dumps({
        "model": MODEL, "stream": False, "format": "json", "keep_alive": "30m",
        "options": {"num_ctx": 32768, "temperature": 0},
        "messages": [{"role": "user", "content": PROMPT.format(
            entry=json.dumps({k: entry[k] for k in entry if k not in ("place",)}, ensure_ascii=False, indent=1),
            needs=", ".join(entry["needs"]), report=report[:3000] or "(none)", text=text)}],
    }).encode()
    req = urllib.request.Request(args.model_host.rstrip("/") + "/api/chat", body, {"Content-Type": "application/json"})
    for attempt in range(3):
        try:
            msg = json.load(urllib.request.urlopen(req, timeout=1800))["message"]["content"]
            return json.loads(msg)
        except Exception as ex:
            err = ex
            time.sleep(5)
    raise RuntimeError(f"model call failed: {err}")


def get(url, timeout=120):
    req = urllib.request.Request(url, headers={"User-Agent": "aph-sourcing/1.0"})
    return urllib.request.urlopen(req, timeout=timeout).read()


def supplement():
    """sources.py saves archive.org *catalog pages*, not book text. Add the full OCR text for
    unrestricted archive.org items, and download PDF links that the browser couldn't open."""
    for e in load()["entries"]:
        if not e.get("needs") or (args.only and e["id"] not in args.only):
            continue
        dest = OUT / e["id"]
        urls = [e["source"].get("url")] + [a["url"] for a in e.get("see_also", [])]
        for i, u in enumerate(x for x in urls if x):
            m = re.match(r"https?://archive\.org/details/([^/?#]+)", u)
            try:
                if m:
                    ident = m.group(1)
                    out = dest / f"ia_{ident}.txt"
                    if out.exists():
                        continue
                    meta = json.loads(get(f"https://archive.org/metadata/{ident}", 30))
                    if meta.get("metadata", {}).get("access-restricted-item") in ("true", True):
                        continue  # borrow-only: full text isn't downloadable
                    txt = next((f["name"] for f in meta.get("files", []) if f["name"].endswith("_djvu.txt")), None)
                    if txt:
                        dest.mkdir(parents=True, exist_ok=True)
                        out.write_bytes(get(f"https://archive.org/download/{ident}/{urllib.request.quote(txt)}"))
                        print(f"  + full text {ident} ({out.stat().st_size // 1024} KB)")
                elif u.lower().split("?")[0].endswith(".pdf") and not (dest / f"{i}.pdf").exists():
                    dest.mkdir(parents=True, exist_ok=True)
                    (dest / f"{i}.pdf").write_bytes(get(u))
                    print(f"  + pdf {u}")
            except Exception as ex:
                print(f"  ! {u}: {ex.__class__.__name__}")


def main():
    OUT.mkdir(exist_ok=True)
    if not args.dry_run:
        bdir = ROOT / "data" / "backups"
        bdir.mkdir(exist_ok=True)
        b = bdir / f"aphorisms.{datetime.now():%Y%m%d-%H%M%S}.json"
        shutil.copy2(DATA, b)
        print("backup:", b)

    if not args.skip_fetch:
        print("fetching sources (this can take a while)...")
        subprocess.run([sys.executable, str(ROOT / "tools" / "sources.py"), "fetch", *args.only], cwd=ROOT)

    supplement()

    ok, out = check()
    if not ok and not args.dry_run:
        sys.exit("data check fails BEFORE any changes; fix that first:\n" + out)

    todo = [e for e in load()["entries"] if e.get("needs") and (not args.only or e["id"] in args.only)]
    changes, leftovers = [], []

    for n, e in enumerate(todo, 1):
        eid = e["id"]
        print(f"[{n}/{len(todo)}] {eid} ({', '.join(e['needs'])})")
        text, report = source_text(eid)
        if not e["source"].get("url") and not e.get("see_also"):
            leftovers.append((eid, e["needs"], "No URL to fetch. See SOURCING.md group 5 for where to look."))
            continue
        if len(norm(text)) < 200:
            why = "Nothing usable was saved"
            if "FAILED" in report: why += " (fetch failed: blocked, timed out, or site down)"
            leftovers.append((eid, e["needs"], why + ". Open the source by hand; see sources/%s/REPORT.md." % eid))
            continue
        try:
            r = ask(e, report, window(text, e))
        except Exception as ex:
            leftovers.append((eid, e["needs"], str(ex)))
            continue

        nt = norm(text)
        evidence_ok = bool(r.get("evidence")) and norm(r["evidence"]) in nt and len(norm(r["evidence"])) > 15
        fixes = [f for f in r.get("quote_fixes") or [] if isinstance(f, dict) and isinstance(f.get("index"), int)
                 and 0 <= f["index"] < len(e["quotes"]) and f.get("orig")]
        fixes_ok = all(norm(f["orig"]) in nt for f in fixes)
        resolved = [x for x in r.get("resolved_needs") or [] if x in e["needs"]]

        if r.get("status") not in ("verified", "corrected") or not resolved or not evidence_ok or not fixes_ok:
            why = r.get("reason") or ""
            if r.get("status") in ("verified", "corrected"):
                why = ("Model claimed a match but its evidence/wording isn't in the saved text, so it was not applied. "
                       f"Model said: {r.get('evidence', '')[:200]!r}")
            leftovers.append((eid, e["needs"], why or "Model couldn't confirm from the saved text."))
            continue

        d = load()
        cur = next(x for x in d["entries"] if x["id"] == eid)
        before = json.dumps({k: cur.get(k) for k in ("quotes", "ref", "needs", "provenance_notes")}, ensure_ascii=False, indent=1)
        for f in fixes:
            cur["quotes"][f["index"]]["orig"] = f["orig"]
        if r.get("ref"):
            cur["ref"] = r["ref"]
        cur["needs"] = [x for x in cur["needs"] if x not in resolved]
        if r.get("provenance_note"):
            cur["provenance_notes"] = r["provenance_note"]
        after = json.dumps({k: cur.get(k) for k in ("quotes", "ref", "needs", "provenance_notes")}, ensure_ascii=False, indent=1)
        entry_log = f"## `{eid}` ({r['status']})\n\nEvidence: > {r['evidence'][:400]}\n\nBefore:\n```json\n{before}\n```\nAfter:\n```json\n{after}\n```\n"

        if args.dry_run:
            changes.append("(dry run, not applied) " + entry_log)
            continue
        old = DATA.read_text(encoding="utf-8")
        save(d)
        ok, out = check()
        if ok:
            changes.append(entry_log)
            if cur["needs"]:
                leftovers.append((eid, cur["needs"], "Partly resolved; the rest still needs checking."))
        else:
            DATA.write_text(old, encoding="utf-8")
            leftovers.append((eid, e["needs"], "Patch failed the data check and was reverted:\n```\n" + out[-600:] + "\n```"))

    (OUT / "CHANGES.md").write_text(f"# Changes ({datetime.now():%Y-%m-%d %H:%M})\n\n" + ("\n".join(changes) or "None.\n"), encoding="utf-8")
    lines = [f"# Leftovers ({datetime.now():%Y-%m-%d %H:%M})", "", "| id | needs | why |", "|---|---|---|"]
    lines += [f"| `{i}` | {', '.join(nd)} | {w.replace('|', '/').replace(chr(10), ' ')} |" for i, nd, w in leftovers]
    (OUT / "LEFTOVERS.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"\n{len(changes)} changed, {len(leftovers)} left over -> sources/CHANGES.md, sources/LEFTOVERS.md")


if __name__ == "__main__":
    main()
