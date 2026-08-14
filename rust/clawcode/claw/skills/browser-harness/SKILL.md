---
name: browser-harness
description: Use when automating browser interactions (open pages, click, type, screenshot), extracting content from anti-scraping sites (Cloudflare, bot detection), or using remote cloud browsers.
---

# Browser Harness — Browser Automation & Interaction Skill

Operational guide for the `browser-harness` CLI tool covering web page browsing, screenshots, clicking, form filling, web scraping, remote cloud browsers, and anti-scraping content extraction.

> `browser-harness` is already in PATH (`C:\Users\%USERNAME%\.local\bin\browser-harness.exe`). Use directly — no installation check needed.

## When to Use

Use this skill when **any** of the following apply:
1. **Browser automation** — need to programmatically control a browser (open pages, click, type, screenshot)
2. **Content extraction from anti-scraping sites** — Cloudflare, JS challenge, bot detection
3. **UI testing / interaction** — need to fill forms, click buttons, handle dialogs via coordinates
4. **Remote cloud browsers** — need concurrent or persistent browser sessions
5. **Network monitoring** — need to capture network requests made by page

## How to Use

Two recommended approaches, **neither has quoting conflicts**. Quick comparison:

| Approach | When to Use | Speed |
|----------|-------------|-------|
| **A. bash script** | Script reuse, complex operations | Fastest |
| **B. `--stdin`** | Ad-hoc, no bash available | Zero files |

### Approach A: bash script (fastest)

Write a `.sh` file with bash single quotes `-c '...'` — clean quoting, no conflicts:

```bash
# open_news.sh
browser-harness -c '
new_tab("https://news.qq.com")
wait_for_load()
print(js("document.title"))
'
```

```powershell
bash open_news.sh
```

### Approach B: `--stdin` pipe (works in any shell)

Code passes via stdin, **no quoting issues on the command line**:

```powershell
# PowerShell
@'
new_tab("https://news.qq.com")
wait_for_load()
print(js("document.title"))
'@ | browser-harness --stdin
```

```bash
# bash / WSL
browser-harness --stdin << 'EOF'
new_tab("https://news.qq.com")
wait_for_load()
print(js("document.title"))
EOF
```

> First page open must use `new_tab(url)`, not `goto_url(url)`.
> `goto_url` navigates the current tab; if it's a `chrome://` page it will fail.

### js() quoting tips (universal)

```python
# CSS selector (avoids quote nesting)
js("document.querySelector('#stepDisplay').textContent")

# Reference page globals directly
js("stepDisp.textContent")
js("state.player")

# JSON.stringify returns a string — safest approach
js("JSON.stringify(state.player)")

# Template literals with backticks
js("`Steps: ${stepDisp.textContent}`")
```

> `js('JSON.stringify(...)')` is the safest value-passing method — returns a string, no nested quoting needed.

## Key Capabilities Overview

- **new_tab / goto_url**: Open and navigate pages
- **capture_screenshot**: Viewport or full-page screenshots
- **click_at_xy**: Coordinate-based clicking (bypasses iframe/Shadow DOM issues)
- **type_text / press_key**: Keyboard input
- **js()**: Execute arbitrary JavaScript in page context
- **cdp()**: Direct Chrome DevTools Protocol access
- **NetworkMonitor**: Capture HTTP requests
- **readwebfetch**: Extract article content from anti-scraping sites (Cloudflare, etc.)
- **start_remote_daemon**: Cloud browser for concurrent tasks
- **PDF export, multi-tab management, alert handling**

---

## 1. Opening Pages

```python
new_tab("https://news.ycombinator.com")   # Open in new tab
wait_for_load()                            # Wait for page load
print(page_info())                         # Print page info
```

Effect: Opens a new tab, loads Hacker News, prints title/URL/viewport.

```python
goto_url("https://example.com/page2")     # Navigate current tab
```

> Use `new_tab` for first open, `goto_url` for subsequent navigation (no new tab created).

---

## 2. Screenshots

```python
capture_screenshot()                       # Capture current viewport, auto-send to AI
capture_screenshot("/tmp/shot.png")        # Save to file
capture_screenshot(max_dim=1800)           # Limit dimensions to avoid model rejection
capture_screenshot(full=True)              # Full page (including below fold)
```

Effect: Screenshot lets the AI "see" the page. Always screenshot first, then decide.

> Screenshots are in device pixels, click coordinates are in CSS pixels. On 2× displays, check `js("window.devicePixelRatio")` first and scale accordingly.

---

## 3. Clicking

```python
# 1. Screenshot first — locate the target
capture_screenshot()

# 2. Calculate coordinates, click
click_at_xy(450, 320)                      # Click at (450, 320)

# 3. Screenshot again — confirm the result
capture_screenshot()
```

Effect: First screenshot shows the button position → mouse clicks on it → second screenshot confirms the page changed.

> Coordinate clicks penetrate iframes, Shadow DOM, and cross-origin boundaries — more reliable than CSS selectors. Only use DOM manipulation for hidden elements (0×0 nodes).

---

## 4. Form Filling

```python
# Click into the input field first
click_at_xy(300, 400)
# Then type
type_text("hello world")
# Submit
press_key("Enter")
```

Effect: Mouse clicks the search box → types "hello world" → presses Enter to search.

```python
# Or fill directly with JS
js("document.querySelector('input').value = 'hello'")
```

---

## 5. Getting Page Text

```python
print(page_info())                         # Title + URL + viewport
print(js("document.body.innerText"))       # All page text
print(js("document.title"))                # Page title
```

Effect: Get page content directly without needing a screenshot.

---

## 6. Executing Arbitrary JavaScript

```python
# Get data
data = js("""
  JSON.stringify({
    title: document.title,
    links: [...document.querySelectorAll('a')].map(a => a.href)
  })
""")

# Modify page
js("document.querySelector('.ad-banner')?.remove()")
js("document.body.style.background = 'white'")

# Call APIs
result = js("""
  (async () => {
    const r = await fetch('/api/data');
    return r.json();
  })()
""")
```

Effect: Run JS in the page context — read data, modify styles, call APIs, just like DevTools Console.

---

## 7. Dialog Handling

```python
# Scenario: clicking a button triggers alert
click_at_xy(200, 300)
# Dialog appears, JS is frozen
cdp("Page.handleJavaScriptDialog", accept=True)   # Click "OK"
```

Effect: When `alert()` / `confirm()` / `beforeunload` dialogs appear, dismiss them at the CDP level — invisible to the user, undetectable by anti-bot.

To suppress all dialogs preemptively:
```python
js("""
window.alert=m=>{};           # Silence alerts
window.confirm=m=>true;       # Auto-confirm
window.onbeforeunload=null;   # Disable leave confirmation
""")
```

---

## 8. Multi-tab Management

```python
# Scenario: switching between multiple pages
tab1 = new_tab("https://a.com")            # Open first
tab2 = new_tab("https://b.com")            # Open second
switch_tab(tab1)                            # Switch back to first
cdp("Target.activateTarget", targetId=tab1) # Bring to foreground (optional)

# List all tabs
for t in list_tabs():
    print(t["url"][:60])
```

---

## 9. Waiting for Page Load

```python
wait_for_load()                            # Wait for page to finish loading
wait_for_text("Login")                     # Wait for text to appear (max 10s)
```

---

## 10. Network Request Capture

```python
# Scenario: verify backend received form submission
from browser_harness.helpers import NetworkMonitor
monitor = NetworkMonitor()

fill_form({"name": "Zhang San", "email": "a@b.com"})
click_at_xy(500, 600)

requests = monitor.get_requests()          # Get captured network requests
```

---

## 11. Scrolling

```python
# Scenario: long page, scroll to bottom to load more
js("window.scrollTo(0, document.body.scrollHeight)")
wait_for_load()
capture_screenshot()                       # Confirm new content appeared
```

---

## 12. PDF Export

```python
# Scenario: save current page as PDF
cdp("Page.printToPDF", landscape=False, printBackground=True)
```

---

## 13. Keyboard Operations

```python
press_key("Enter")                         # Enter
press_key("Tab")                           # Tab
press_key("Escape")                        # Escape
type_text("search keyword")                # Type text sequentially
```

---

## 14. Debugging Tips

```python
# Stuck and don't know the state
print(page_info())                         # Check title/URL/viewport
print(current_tab())                       # Check which tab is attached
tabs = list_tabs()                         # List all tabs
ensure_real_tab()                          # Fix attachment to phantom tab
```

**Common Issues Quick Reference:**

| Symptom | Cause | Solution |
|---------|-------|----------|
| Blank screenshot | Attached to omnibox phantom tab | `ensure_real_tab()` |
| Click does nothing | Wrong coordinates / missed target | Re-screenshot, recalculate, or use `js` |
| Page frozen | Dialog blocking JS | `cdp("Page.handleJavaScriptDialog", accept=True)` |
| Link click no navigation | `beforeunload` blocking | `cdp("Page.handleJavaScriptDialog", accept=True)` |
| Can't get data | Login required | Ask user to login, or `sync_local_profile` |
| `js()` SyntaxError | PowerShell ate the double quotes | Use `--stdin` or bash script approach |
| `page_info()` title has emoji | browser-harness auto-injection, normal | Ignore |
| Sequential moves don't work | Wall/box blocking | `print(js('JSON.stringify(state)'))` check state |
| `steps--` goes negative | Won't happen — `undo()` has `history.length` guard | But `undo` doesn't trigger win state reset |

---

## 15. Remote Cloud Browsers

For **Browser Use Cloud** only — suitable for concurrent subtasks or maintenance-free operation.

```python
start_remote_daemon("work")                # Start a cloud browser
start_remote_daemon("work", proxyCountryCode=None)  # Disable proxy
```

```bash
BU_NAME=work browser-harness -c '
new_tab("https://example.com")
print(page_info())
'
```

```python
stop_remote_daemon("work")                 # Stop, billing stops
```

Start with login state:
```python
list_cloud_profiles()                      # List stored cloud profiles
sync_local_profile("My Chrome Profile")    # Upload local cookies
start_remote_daemon("work", profileName="My Chrome Profile")
```

---

## 16. readwebfetch — Bypass Anti-Scraping

**Scenario:** Site has anti-scraping (Cloudflare, JS challenge, bot detection), regular HTTP requests fail.  
**How it works:** Extracts content via Readability.js in a real browser — no HTTP request, anti-bot can't detect it.

**Prerequisite:** browser-harness auto-loads the `read_webfetch` extension when launching Chromium (`--load-extension`).

```python
d = readwebfetch("https://blog-link.com")
print(d["title"])
print(d["text"][:500])
```

**Return structure:**

| Field | Description |
|-------|-------------|
| `url` | Page URL |
| `title` | Page title |
| `text` | Readability-extracted plain text |
| `excerpt` | Summary |
| `byline` | Author |

**Execution:**

```bash
# bash script
browser-harness -c '
d = readwebfetch("https://blog.csdn.net/...")
print(d["title"])
print("Total " + str(len(d["text"])) + " chars")
'
```

```powershell
# PowerShell
@'
d = readwebfetch("https://blog.csdn.net/...")
print(d["title"])
print("Total " + str(len(d["text"])) + " chars")
'@ | browser-harness --stdin
```

---

## Windows PowerShell Notes

- Use double quotes `"..."` for `-c` argument, single quotes `'...'` inside Python
- Prefer `querySelector('#id')` over `getElementById("id")` to avoid quote nesting
- Use `JSON.stringify(...)` for safe data transfer from js()
- For complex scripts, write a `.py` file and pipe via `Get-Content`
