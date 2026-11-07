// GET /auth/login?return=/path: reached only after Cloudflare Access has signed the reader in
// (email + one-time PIN); send them back to where they were.

export function onRequestGet({ request }) {
  const url = new URL(request.url);
  const back = url.searchParams.get("return") || "/";
  // Only same-site paths, never an open redirect.
  const safe = back.startsWith("/") && !back.startsWith("//") ? back : "/";
  return Response.redirect(new URL(safe, url.origin).toString(), 302);
}
