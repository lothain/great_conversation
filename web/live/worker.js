// The live room: a Durable Object holding every open page's WebSocket.
//   /connect   a page joins (WebSocket upgrade); pages only listen
//   /announce  the Pages Functions broadcast { type: "posts" | "typing", ... } to everyone
// Uses WebSocket hibernation, so an idle room costs nothing.
import { DurableObject } from "cloudflare:workers";

export class Live extends DurableObject {
  async fetch(request) {
    const url = new URL(request.url);
    if (url.pathname === "/connect") {
      const [client, server] = Object.values(new WebSocketPair());
      this.ctx.acceptWebSocket(server);
      return new Response(null, { status: 101, webSocket: client });
    }
    if (url.pathname === "/announce" && request.method === "POST") {
      const text = await request.text();
      for (const ws of this.ctx.getWebSockets()) { try { ws.send(text); } catch {} }
      return new Response("ok");
    }
    return new Response("not found", { status: 404 });
  }
  // Keep-alive pings get a pong; anything else from a page is dropped.
  async webSocketMessage(ws, message) { if (message === "ping") ws.send("pong"); }
  async webSocketClose(ws, code) { try { ws.close(code, "bye"); } catch {} }
}

// The worker itself serves nothing; the Pages project reaches the Durable Object directly.
export default { fetch: () => new Response("great-conversation live room", { status: 404 }) };
