/**
 * brewcode gateway — Cloudflare Worker
 *
 * An OpenAI-compatible proxy that sits in front of Ollama Cloud. The real
 * Ollama Cloud API key lives here as a Worker secret and never ships inside
 * the brewcode binary. brewcode authenticates to this Worker with a separate
 * GATEWAY_TOKEN, which is cheap to rotate if it ever leaks.
 *
 *   brewcode --(Bearer GATEWAY_TOKEN)--> Worker --(Bearer OLLAMA_API_KEY)--> ollama.com
 *
 * Secrets (set once with wrangler — see README.md):
 *   OLLAMA_API_KEY   real Ollama Cloud key (https://ollama.com/settings/keys)
 *   GATEWAY_TOKEN    token brewcode sends; rotate this if it leaks
 */

const OLLAMA_ORIGIN = "https://ollama.com";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    if (url.pathname === "/" || url.pathname === "/health") {
      return new Response("brewcode gateway ok\n", {
        status: 200,
        headers: { "content-type": "text/plain; charset=utf-8" },
      });
    }

    // Only the OpenAI-compatible API surface is proxied.
    if (!url.pathname.startsWith("/v1/")) {
      return gatewayError("not found", 404);
    }

    if (!env.OLLAMA_API_KEY || !env.GATEWAY_TOKEN) {
      return gatewayError("gateway misconfigured: secrets are not set", 500);
    }

    // Authenticate the caller against GATEWAY_TOKEN.
    const presented = (request.headers.get("authorization") || "")
      .replace(/^Bearer\s+/i, "")
      .trim();
    if (presented !== env.GATEWAY_TOKEN) {
      return gatewayError("unauthorized: invalid gateway token", 401);
    }

    // Forward to Ollama Cloud with the real key swapped in.
    const headers = new Headers(request.headers);
    headers.set("authorization", `Bearer ${env.OLLAMA_API_KEY}`);
    headers.delete("host");
    headers.delete("content-length");

    const init = { method: request.method, headers };
    if (request.method !== "GET" && request.method !== "HEAD") {
      init.body = await request.arrayBuffer();
    }

    let upstream;
    try {
      upstream = await fetch(OLLAMA_ORIGIN + url.pathname + url.search, init);
    } catch (err) {
      return gatewayError(`upstream request failed: ${err}`, 502);
    }

    // Pass the response straight back — streaming (SSE) bodies included.
    return new Response(upstream.body, upstream);
  },
};

function gatewayError(message, status) {
  return new Response(
    JSON.stringify({ error: { message, type: "gateway_error" } }),
    { status, headers: { "content-type": "application/json" } },
  );
}
