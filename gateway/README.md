# brewcode gateway

An OpenAI-compatible proxy (Cloudflare Worker) that sits in front of Ollama Cloud.

**Why it exists:** it holds your real Ollama Cloud API key server-side, so the
key never ships inside the brewcode binary. brewcode talks to this Worker with a
separate `GATEWAY_TOKEN` — if that token leaks, you rotate it cheaply and your
Ollama key is never exposed.

```
brewcode --(Bearer GATEWAY_TOKEN)--> Worker --(Bearer OLLAMA_API_KEY)--> ollama.com
```

## Prerequisites

- A free Cloudflare account
- Node.js installed (for `npx wrangler`)
- Your Ollama Cloud API key: https://ollama.com/settings/keys

## Deploy

Run these from this `gateway/` directory:

1. Log in to Cloudflare:
   ```
   npx wrangler login
   ```
2. Set the two secrets (you are prompted to paste each value):
   ```
   npx wrangler secret put OLLAMA_API_KEY
   npx wrangler secret put GATEWAY_TOKEN
   ```
   - `OLLAMA_API_KEY` — your real Ollama Cloud key.
   - `GATEWAY_TOKEN` — any random string you choose. Generate one with
     `openssl rand -hex 24`. This is the token brewcode will send.
3. Deploy:
   ```
   npx wrangler deploy
   ```
   Wrangler prints the URL, e.g.
   `https://brewcode-gateway.<your-subdomain>.workers.dev`.

## Test it

Health check:
```
curl https://brewcode-gateway.<your-subdomain>.workers.dev/health
```

A real chat completion (replace `<GATEWAY_TOKEN>` and the URL):
```
curl https://brewcode-gateway.<your-subdomain>.workers.dev/v1/chat/completions \
  -H "Authorization: Bearer <GATEWAY_TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{"model":"kimi-k2.6","messages":[{"role":"user","content":"say hi"}],"stream":false}'
```
Expect a normal chat-completion JSON response.

## Point brewcode at the gateway

In brewcode's `.env`, replace the direct Ollama Cloud block with:
```
OPENAI_API_KEY=<GATEWAY_TOKEN>
OPENAI_BASE_URL=https://brewcode-gateway.<your-subdomain>.workers.dev/v1
```
Then `brewcode --model "openai/kimi-k2.6"` works with **no Ollama key on the
client**. (Keep the `openai/` prefix — see the main provider docs.)

## Rotating the token

If `GATEWAY_TOKEN` leaks: set a new value with
`npx wrangler secret put GATEWAY_TOKEN`, redeploy, and update clients. Your
Ollama key is never touched.

## Notes

- The Worker **requires** `GATEWAY_TOKEN` — it is not an open proxy.
- Streaming responses (SSE) pass straight through, so streaming chat
  completions work.
- For abuse protection at scale, add a Cloudflare Rate Limiting rule on the
  Worker route (Cloudflare dashboard → Security).
- A leaked `GATEWAY_TOKEN` is recoverable; a leaked Ollama key is not — that is
  the whole point of running this gateway.
