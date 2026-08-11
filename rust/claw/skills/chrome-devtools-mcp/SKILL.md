---
name: chrome-devtools-mcp
description: Use when browsing web pages, extracting content from restricted sites (login walls, paywalls), debugging JS errors, analyzing network requests, or running performance audits via browser DevTools.
---

# Chrome DevTools MCP — Web Browsing & Debugging Skill

Operation guide for the `chrome-devtools-mcp` toolset covering web browsing, interactive debugging, content extraction, and performance analysis.

## When to Use

Use this skill when **any** of the following apply:
1. **Browsing** — need to navigate web pages, extract content, bypass login walls/paywalls
2. **Debugging** — need to inspect console errors, network requests, DOM elements, or page performance
3. **Content extraction** — need to extract article text from restricted pages (Zhihu, CSDN, etc.)
4. **Interaction** — need to fill forms, click elements, handle dialogs on web pages
5. **Performance** — need to run Lighthouse audits, trace performance, or capture heap snapshots

## Core Workflow

```
1. new_page(url) / navigate_page(url)   → Open/navigate to page
2. wait_for(["keyword"])                  → Wait for content to load
3. take_snapshot()                        → Get element structure (uid)
4. take_screenshot()                      → Confirm visual state
5. evaluate_script(() => ...)            → Execute JS / extract data
6. list_console_messages()                → Check console errors
```

## Key Capabilities

- **Bypass restrictions**: Remove login/paywall overlays, unlock copy restrictions, expand truncated articles
- **Debug JS errors**: List and inspect console messages, identify uncaught exceptions
- **Network analysis**: List network requests, inspect request/response bodies
- **DOM interaction**: Click, fill, type, hover, drag — all via accessibility tree (uid)
- **Performance**: Lighthouse audits, performance traces, memory heap snapshots
- **Device emulation**: Mobile viewport, user agent switching

---

# Part 1 — Browsing & Restriction Bypass

Based on `chrome-devtools-mcp` toolset for bypassing login walls, copy restrictions, and paywall overlays on sites like Zhihu, CSDN.

## Standard Browsing Flow

```
Step 1: new_page(url)          → Open page
Step 2: wait_for(["keyword"])  → Wait for content load
Step 3: take_snapshot()        → Get accessibility tree (text structure)
Step 4: take_screenshot()      → Confirm visual state (optional)
Step 5: evaluate_script()      → Extract specific data
```

## Restriction Bypass Guide

### 0. Standard Detect-Remove-Extract Pattern

```javascript
// Step 1: Detect
evaluate_script(() => {
  JSON.stringify({
    hasMask: !!document.querySelector('[class*="mask"], [class*="overlay"], [class*="passport"]'),
    hasReadMore: !!document.querySelector('.btn-readmore, [class*="readmore"], [class*="expand"]'),
    articleLen: document.querySelector('article')?.innerText.length || 0,
    title: document.title
  })
})

// Step 2: Remove mask
evaluate_script(() => {
  document.querySelectorAll('[class*="mask"], [class*="overlay"], [class*="passport"], [class*="login"], [class*="modal"], .hide-article-box')
    .forEach(el => el.remove());
  document.body.style.overflow = 'auto';
  document.body.style.position = '';
  const a = document.querySelector('article');
  if (a) { a.style.height = 'auto'; a.style.maxHeight = 'none'; }
})

// Step 3: Extract content
evaluate_script(() => {
  const a = document.querySelector('article') || document.querySelector('[class*="content"]') || document.querySelector('[class*="article"]');
  return a?.innerText || 'not found';
})
```

### 1. Bypass Login Wall / Paywall Overlay

```javascript
// Remove overlay elements
evaluate_script(() => {
  document.querySelectorAll('.login-guard, .pay-wall, .modal-mask, [class*="mask"], [class*="overlay"]')
    .forEach(el => el.remove());
})
```

```javascript
// Remove body scroll lock and show content
evaluate_script(() => {
  document.body.style.overflow = 'auto';
  document.querySelectorAll('.login-guard, .pay-wall, .sign-in, .modal, .overlay')
    .forEach(el => el.remove());
  // Restore hidden content
  document.querySelectorAll('[class*="content"], [class*="article"], [class*="main"]')
    .forEach(el => el.style.display = 'block');
})
```

### 2. Unlock Copy Restrictions

```javascript
evaluate_script(() => {
  document.addEventListener('copy', e => e.stopPropagation(), true);
  document.addEventListener('selectstart', e => e.stopPropagation(), true);
  document.body.style.userSelect = 'auto';
  document.querySelectorAll('*').forEach(el => el.style.userSelect = 'auto');
})
```

### 3. Extract Truncated Full Text

```javascript
// Standard flow: detect → remove mask → extract
evaluate_script(() => {
  const hasMask = !!document.querySelector('[class*="mask"], [class*="overlay"], [class*="passport"]');
  const hasReadMore = !!document.querySelector('.btn-readmore, [class*="readmore"], [class*="expand"]');
  return JSON.stringify({hasMask, hasReadMore, articleLen: document.querySelector('article')?.innerText.length || 0});
})

// If read-more button exists, click it first
evaluate_script(() => {
  const btn = [...document.querySelectorAll('button, a, span, div')]
    .find(el => el.textContent.includes('展开阅读全文') || el.textContent.includes('全文'));
  btn?.click();
})
```

```javascript
// Zhihu — expand full text
evaluate_script(() => {
  const btn = [...document.querySelectorAll('button, a, span')]
    .find(el => el.textContent.includes('展开阅读全文') || el.textContent.includes('全文'));
  if (btn) btn.click();
})
```

```javascript
// CSDN — remove login overlay + extract full text (verified 2026)
evaluate_script(() => {
  document.querySelectorAll('.mask, .mask-dark, .passport-login-tip-container, .passport-login-container, .passport-login-box, .passport-login-mark, .hide-article-box')
    .forEach(el => el.remove());
  document.body.style.overflow = 'auto';
  document.body.style.position = '';
  const article = document.querySelector('article') || document.querySelector('.article_content');
  if (article) {
    article.style.setProperty('height', 'auto', 'important');
    article.style.setProperty('max-height', 'none', 'important');
  }
})

// Extract content
evaluate_script(() => {
  const art = document.querySelector('article') || document.querySelector('.article_content') || document.querySelector('#article_content');
  return 'Title: ' + document.title + '\n\n' + art.innerText;
})
```

### 4. Extract Page Text

```javascript
// Get article plain text
evaluate_script(() => {
  const article = document.querySelector('article') ||
    document.querySelector('[class*="content"]') ||
    document.querySelector('[class*="article"]') ||
    document.querySelector('main');
  return article ? article.innerText : document.body.innerText;
})
```

```javascript
// Get all page text (preserving structure)
evaluate_script(() => {
  return [...document.querySelectorAll('h1, h2, h3, p, li, pre, code')]
    .map(el => el.tagName + ': ' + el.innerText.trim())
    .filter(s => s.length > 3)
    .join('\n---\n');
})
```

### 5. Zhihu-Specific Bypass

```javascript
evaluate_script(() => {
  // Close dialog
  document.querySelector('.Modal-closeButton, button[class*="close"]')?.click();
  document.querySelector('[class*="signIn"], [class*="Modal"]')?.remove();
  // Expand all collapsed answers
  document.querySelectorAll('.RichContent.is-collapsed').forEach(el => {
    el.classList.remove('is-collapsed');
    el.style.height = 'auto';
    el.style.maxHeight = 'none';
    el.style.overflow = 'visible';
  });
  document.body.style.overflow = 'auto';
})
```

### 6. WeChat Public Account Articles (Sogou Gateway)

WeChat public account articles are normally login-gated in browsers, but Sogou WeChat Search (the official content index) allows direct access.

```javascript
// Step 1: Search for articles
navigate_page('https://weixin.sogou.com/weixin?type=2&s_from=input&query=' + encodeURIComponent('search keyword'))

// Step 2: Get result list
evaluate_script(() => {
  const items = [...document.querySelectorAll('.news-list2 .wx-rb, .news-list2 li')].filter(el => el.querySelector('h3 a'));
  return items.slice(0, 10).map(el => ({
    title: el.querySelector('h3 a')?.textContent?.trim(),
    link: el.querySelector('h3 a')?.href,
    source: el.querySelector('.account')?.textContent?.trim(),
    date: el.querySelector('.time')?.textContent?.trim(),
    summary: el.querySelector('.txt-info')?.textContent?.trim()?.slice(0, 80)
  }));
})

// Step 3: Open article link (no login required)
navigate_page('result-link')

// Step 4: Extract content
evaluate_script(() => document.body.innerText)
```

**Verified (2026):** Sogou WeChat Search for `chrome devtools` returns 634 results. Opening the link gives full 2856-character article with no restrictions.

### 7. Mobile Emulation (some sites have fewer restrictions on mobile)

```javascript
emulate({
  userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1',
  viewport: '375x667x2,mobile,touch'
})
```

## Quick Command Reference

| Operation | Tool | Description |
|-----------|------|-------------|
| Open page | `new_page(url)` | Open in new tab |
| Navigate | `navigate_page(url)` | Navigate current tab |
| Wait for content | `wait_for(["text"])` | Wait for text to appear |
| Screenshot | `take_screenshot()` | Full-page screenshot |
| DOM snapshot | `take_snapshot()` | Accessibility tree text structure |
| Execute JS | `evaluate_script(fn)` | Arbitrary JS operations |
| JS with args | `evaluate_script(fn, args)` | Execute with parameters |
| Extract content | `evaluate_script(() => document.body.innerText)` | Plain text extraction |
| Remove element | `evaluate_script(() => el.remove())` | Remove overlay/popup |
| Click element | `click(uid)` | Click by snapshot uid |
| Emulate device | `emulate({userAgent, viewport})` | Switch UA/viewport |
| Scroll | `press_key({key: "Space"})` | Simulate key press |

## FAQ (Practical Experience)

### 1. Popup class names don't match?

First inspect the actual overlay elements:
```javascript
evaluate_script(() => {
  [...document.querySelectorAll('div[style*="fixed"], div[style*="absolute"], [class*="overlay"], [class*="modal"], [class*="mask"], [class*="popup"]')]
    .map(el => ({tag: el.tagName, cls: el.className.slice(0,80), visible: el.offsetParent !== null}))
})
```

### 2. How to tell if content is complete or truncated?

```javascript
evaluate_script(() => {
  const a = document.querySelector('article') || document.querySelector('.Post-RichText');
  const ratio = a.scrollHeight / a.clientHeight;
  JSON.stringify({
    textLen: a.innerText.length,
    scrollH: a.scrollHeight, clientH: a.clientHeight,
    ratio: ratio.toFixed(2), // > 1.2 means overflow hidden
    endText: a.innerText.slice(-100)
  })
})
```

If it ends with `-- The End --`, copyright notice, or a natural ending, it's complete.

### 3. CSDN overlay class names (verified 2026)

| CSDN Class | Description |
|------------|-------------|
| `.mask` + `.mask-dark` | Background overlay |
| `.passport-login-tip-container` | Login prompt bar |
| `.passport-login-container` | Login dialog container |
| `.passport-login-box` / `.passport-login-mark` | Login box and overlay |
| `.hide-article-box` | Article collapse bar |

### 4. Zhihu overlay class names (verified 2026)

| Zhihu Class | Description |
|-------------|-------------|
| `.Modal.Modal--default.signFlowModal` | Login dialog |
| `.signFlowModal-container` | Login container |
| Content selector: `.Post-RichText` or `.RichText` | |

### 5. Short article vs truncated article

- Some articles are genuinely short (many images/code, few words) — e.g., 2081 chars but scrollHeight = 8550px
- Verification: check end for natural termination, or confirm via `document.title`
- Zhihu columns without login may redirect to search page — check `location.href`

### 6. What can vs cannot be bypassed

| Type | Principle | Bypassable? | Example |
|------|-----------|-------------|---------|
| DOM overlay | Content in DOM, hidden behind a div | Yes — just remove it | CSDN, Zhihu columns |
| Lazy load | Content loaded on scroll | Yes — trigger scroll | Most comment sections |
| API auth | Content fetched via cookie-authenticated API | No — no cookie = no data | Bilibili comments, Weibo |
| SSR hidden | Server-rendered but hidden via class | Yes — change style | Juejin paid articles |

### 7. Chrome restart / disconnect handling

MCP mode manages browser lifecycle automatically. CLI mode:
```bash
chrome-devtools stop     # Stop background process
chrome-devtools status   # Check status
```

---

# Part 2 — Debugging Guide

Based on `chrome-devtools-mcp` toolset for debugging web pages, inspecting errors, and analyzing performance.

## Tool Overview

```
Category         Tool                             Purpose
──────           ───                              ───
Navigation       new_page / navigate_page         Open/navigate pages
                 close_page / select_page         Close/switch tabs
                 list_pages                       List all tabs
                 wait_for                         Wait for text

Debugging        evaluate_script                  Execute JS in page
                 take_snapshot                    Get accessibility tree (uid)
                 take_screenshot                  Screenshot
                 list_console_messages            List console logs
                 get_console_message(msgid)       View specific log details
                 lighthouse_audit                 Lighthouse audit

Interaction      click(uid)                       Click element
                 fill(uid, value)                 Fill input field
                 fill_form([{uid,value}])         Batch form fill
                 type_text(text)                  Keyboard input
                 press_key(key)                   Key press (Enter/Tab/Ctrl+A)
                 hover(uid)                       Hover
                 drag(from_uid, to_uid)           Drag
                 handle_dialog(action)            Handle browser dialogs
                 upload_file(path, uid)           Upload file

Network          list_network_requests            List network requests
                 get_network_request(reqid)       View request details/response

Performance      performance_start_trace          Start performance recording
                 performance_stop_trace           Stop + analyze
                 performance_analyze_insight      Analyze specific metric
                 take_memory_snapshot             Heap snapshot

Emulation        emulate({userAgent, viewport})   Simulate device
                 resize_page(width, height)       Resize window
```

## Standard Debugging Flows

### Flow 1: JS Error Investigation

```
1. navigate_page(url)                      → Enter page
2. list_console_messages()                 → View errors
3. get_console_message(msgid)              → View error details
4. evaluate_script(() => { /* fix */ })   → Fix the issue
5. verify
```

### Flow 2: Network Request Analysis

```
1. navigate_page(url)                      → Load page
2. list_network_requests()                 → List all requests
3. get_network_request(reqid)              → View request/response body
4. Identify 404s, CORS errors, slow requests
```

### Flow 3: DOM / Style Debugging

```
1. take_snapshot()                         → Get element structure (with uid)
2. click(uid) / fill(uid, value)           → Interact
3. evaluate_script(() => getComputedStyle(el))  → Check styles
4. evaluate_script(() => { el.style.color = 'red' })  → Temporary modification
5. take_screenshot()                       → Confirm visually
```

### Flow 4: Performance Analysis

```
1. performance_start_trace({reload: true})  → Start recording + reload
2. (wait for page to load)
3. performance_stop_trace()                 → Stop and analyze
4. performance_analyze_insight({insightName, insightSetId})  → Deep dive
```

## Debugging Quick Reference

### Console

```javascript
// View all console messages
list_console_messages({includePreservedMessages: true})

// View specific message
get_console_message({msgid: 0})
```

### Element Inspection

```javascript
// Get interactive elements list (with uid)
take_snapshot()

// Verbose version (more properties)
take_snapshot({verbose: true})

// Inspect element styles
evaluate_script(() => {
  const el = document.querySelector('h1');
  return getComputedStyle(el);
})

// Get element dimensions / position
evaluate_script(() => {
  const el = document.querySelector('h1');
  return el.getBoundingClientRect();
})
```

### Page Interaction

```javascript
// Click (get uid via take_snapshot first)
click({uid: "element-123"})

// Fill input
fill({uid: "input-456", value: "search text"})

// Fill + Enter
fill({uid: "input-456", value: "search text"})
press_key({key: "Enter"})

// Keyboard shortcuts
press_key({key: "Control+A"})
press_key({key: "Control+C"})

// Handle browser dialogs (alert/confirm)
handle_dialog({action: "accept"})
handle_dialog({action: "dismiss"})
```

### Network

```javascript
// View all network requests
list_network_requests({pageSize: 50, resourceTypes: ["XHR", "Fetch", "Document"]})

// View request details
get_network_request({reqid: 0})

// Save response body to file
get_network_request({reqid: 0, responseFilePath: "response.json"})
```

### Memory Debugging

```javascript
// Capture heap snapshot (for memory leak analysis)
take_memory_snapshot({filePath: "heap.heapsnapshot"})
```

### Lighthouse Audit

```javascript
// Accessibility + SEO + Best Practices
lighthouse_audit({device: "desktop"})
lighthouse_audit({device: "mobile"})
lighthouse_audit({mode: "snapshot"}) // No reload, analyze current state
```

## Typical Scenarios

### Scenario A: White Screen / JS Error Fix

```
1. list_console_messages()             → Check for JS errors
2. get_console_message(0)             → View first error details
3. evaluate_script(() => { ... })     → Temporary fix in page
4. Fix in source code, reload, verify
```

### Scenario B: API Endpoint Debugging

```
1. navigate_page('https://example.com')
2. list_network_requests({resourceTypes: ["XHR", "Fetch"]})  → Filter API calls
3. get_network_request(0)  → View request params + response data
```

### Scenario C: Form Submission Verification

```
1. take_snapshot()                    → Get form element uids
2. fill({uid, value})                 → Fill each field
3. click({uid})                       → Click submit button
4. list_network_requests()            → Check if request was sent
5. list_console_messages()            → Check for errors
```

### Scenario D: Responsive Layout Debugging

```
1. emulate({viewport: '375x667x2,mobile,touch'})   → Switch to mobile
2. take_screenshot()                                 → Screenshot for review
3. emulate({viewport: '1280x720'})                   → Switch back to desktop
4. take_screenshot()                                 → Compare results
```
