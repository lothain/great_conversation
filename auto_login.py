"""Log in to archive.org and Gallica with credentials.local.json, saving cookies to the same
profile folder tools/sources.py uses. Run from the project root:

    python auto_login.py                 # uses ./credentials.local.json
    python auto_login.py path/to/creds.json

The browser is visible so you can solve a CAPTCHA or fix a step if a site's form changed.
Untested against the live sites: if a field isn't found, finish that login by hand in the window.
Delete credentials.local.json (and tools/.browser-profile/) when you're done.
"""
import json, sys
from playwright.sync_api import sync_playwright

CREDS = json.load(open(sys.argv[1] if len(sys.argv) > 1 else "credentials.local.json", encoding="utf-8"))
PROFILE = "tools/.browser-profile"

SITES = {
    "archive.org": "https://archive.org/account/login",
    "gallica": "https://gallica.bnf.fr/services/engine/search/sru?operation=searchRetrieve&query=x",  # page with login link
    "jstor": "https://www.jstor.org/action/showLogin",
}


def fill_first(page, selectors, value):
    for sel in selectors:
        loc = page.locator(sel)
        if loc.count() and loc.first.is_visible():
            loc.first.fill(value)
            return True
    return False


def login(page, name):
    c = CREDS[name]
    page.goto(SITES[name], wait_until="domcontentloaded")
    if name == "gallica":
        # Gallica sends you to the BnF authentication page via its "Connexion" link
        for text in ("Se connecter", "Connexion", "Log in", "Sign in"):
            link = page.get_by_text(text, exact=False)
            if link.count():
                link.first.click()
                page.wait_for_load_state("domcontentloaded")
                break
    ok_user = fill_first(page, ['input[name="username"]', 'input[type="email"]', 'input[name="login"]',
                                'input[id*="user" i]', 'input[type="text"]'], c["username"])
    ok_pass = fill_first(page, ['input[name="password"]', 'input[type="password"]'], c["password"])
    if ok_user and ok_pass:
        page.locator('input[type="password"]').first.press("Enter")
        page.wait_for_load_state("networkidle", timeout=30000)
        print(f"[{name}] submitted - check the window that you're logged in")
    else:
        print(f"[{name}] couldn't find the login form - log in by hand in this tab")


with sync_playwright() as p:
    ctx = p.chromium.launch_persistent_context(PROFILE, headless=False)
    for name in CREDS:
        if name in SITES:
            login(ctx.new_page(), name)
    input("\nCheck each tab (borrow your archive.org books now too), then press Enter to save and close...")
    ctx.close()
