# Icons

Phase-2 placeholder. Generate real icons before shipping the PWA. The
manifest expects:

- `icon-192.png` — 192×192, "any" purpose
- `icon-512.png` — 512×512, "any" purpose
- `icon-maskable-512.png` — 512×512, "maskable" purpose (safe zone padded)

Until they exist, the manifest just 404s on the icon URLs and the page
still works — it just won't pass a Lighthouse PWA audit.
