/**
 * local-dev-server.mjs — LOCAL TESTING ONLY.
 *
 * A plain Node stand-in for the Cloudflare Worker in src/worker.js. It mirrors
 * the same logic (validate GATEWAY_TOKEN, swap in OLLAMA_API_KEY, proxy to
 * Ollama Cloud) so the end-user flow can be tested without deploying.
 *
 * Production uses src/worker.js on Cloudflare — NOT this file.
 *
 * Run:  node local-dev-server.mjs     (reads secrets from .dev.vars)
 */

import { createServer } from "node:http";
import { readFileSync } from "node:fs";

const OLLAMA_ORIGIN = "https://ollama.com";
const PORT = Number(process.env.PORT) || 8787;

// Load secrets from .dev.vars (the same file `wrangler dev` would use).
const vars = Object.fromEntries(
  readFileSync(new URL(".dev.vars", import.meta.url), "utf8")
    .split(/\r?\n/)
    .filter((line) => line.includes("=") && !line.trimStart().startsWith("#"))
    .map((line) => {
      const i = line.indexOf("=");
      return [line.slice(0, i).trim(), line.slice(i + 1).trim()];
    }),
);
const { OLLAMA_API_KEY, GATEWAY_TOKEN } = vars;
if (!OLLAMA_API_KEY || !GATEWAY_TOKEN) {
  console.error("missing OLLAMA_API_KEY or GATEWAY_TOKEN in .dev.vars");
  process.exit(1);
}

const sendJson = (res, status, obj) => {
  res.writeHead(status, { "content-type": "application/json" });
  res.end(JSON.stringify(obj));
};

createServer(async (req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);

  if (url.pathname === "/" || url.pathname === "/health") {
    res.writeHead(200, { "content-type": "text/plain" });
    res.end("brewcode gateway ok (local-dev-server)\n");
    return;
  }
  if (!url.pathname.startsWith("/v1/")) {
    return sendJson(res, 404, { error: { message: "not found" } });
  }

  const presented = (req.headers["authorization"] || "")
    .replace(/^Bearer\s+/i, "")
    .trim();
  if (presented !== GATEWAY_TOKEN) {
    console.log(`${req.method} ${url.pathname} -> 401 (invalid gateway token)`);
    return sendJson(res, 401, {
      error: { message: "unauthorized: invalid gateway token" },
    });
  }

  const chunks = [];
  for await (const c of req) chunks.push(c);
  const body = chunks.length ? Buffer.concat(chunks) : undefined;

  let upstream;
  try {
    upstream = await fetch(OLLAMA_ORIGIN + url.pathname + url.search, {
      method: req.method,
      headers: {
        authorization: `Bearer ${OLLAMA_API_KEY}`,
        "content-type": req.headers["content-type"] || "application/json",
      },
      body,
    });
  } catch (err) {
    return sendJson(res, 502, { error: { message: `upstream failed: ${err}` } });
  }

  res.writeHead(upstream.status, {
    "content-type": upstream.headers.get("content-type") || "application/json",
  });
  for await (const part of upstream.body) res.write(part);
  res.end();
  console.log(`${req.method} ${url.pathname} -> ${upstream.status}`);
}).listen(PORT, () => {
  console.log(`[local-dev-server] brewcode gateway stand-in on http://localhost:${PORT}`);
});
