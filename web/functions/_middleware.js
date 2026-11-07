// Every deploy also gets its own address (https://<id>.great-conversation.pages.dev). Sign-in
// (Cloudflare Access) only guards the real address, so on those copies /auth/* isn't protected,
// no one can sign in, and /auth/api/me answers 401. Send visitors to the real address instead.
const CANONICAL = "great-conversation.pages.dev";

export async function onRequest({ request, next }) {
  const url = new URL(request.url);
  if (url.hostname.endsWith("." + CANONICAL)) {
    url.hostname = CANONICAL;
    return Response.redirect(url.toString(), 302);
  }
  return next();
}
