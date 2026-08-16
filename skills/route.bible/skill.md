---
name: route-bible
description: "Reference Bible passages via app-agnostic links and QR codes."
version: 1.0.0
author: Dylan Shade (dpshde)
license: MIT
platforms: [linux, macos, windows]
metadata:
  hermes:
    tags: [Bible, Religion, Scripture, Routing, API]
---

# route.bible

Reference Bible passages and generate app-agnostic links or QR codes. route.bible acts as a universal router, allowing users to open scriptures in their preferred Bible application (YouVersion, BibleGateway, Logos, etc.).

## Quick Reference

| Action | Pattern |
|--------|---------|
| Generate link | `https://route.bible/GEN.1.1` |
| Generate QR code | `https://route.bible/GEN.1.1?qr=true` |

## Usage

The routing layer follows a stable URL contract using standard book abbreviations and OSIS-style references.

### Canonical Link Generation

To generate a universal link for a passage, use the following URL pattern:
`https://route.bible/<BOOK>.<CHAPTER>.<VERSE>`

Examples:
- `https://route.bible/GEN.1.1`
- `https://route.bible/JHN.3.16`

### QR Code Integration

To generate a QR code pointing to a passage, append `?qr=true` to the canonical link.
- `https://route.bible/ROM.12.1-2?qr=true`
