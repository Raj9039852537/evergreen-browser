# Evergreen Browser userscripts

Place `*.user.js` (or other `.js` files containing a `==UserScript==` metadata block) in this directory. The browser loads scripts at startup; restart Evergreen after adding or editing a script.

Supported metadata in this initial implementation:

- `@name`
- `@match` (wildcard matching)
- `@include` (treated as a wildcard match)
- `@grant GM_xmlhttpRequest` / `@grant GM.xmlHttpRequest`
- `@connect hostname` (exact host, `*.example.com`, or `*`)
- `@run-at` is parsed; initial implementation injects scripts at document start.

`GM_xmlhttpRequest` supports method, URL, headers, string data, `onload`, `onerror`, `onreadystatechange`, and `onloadend`. Requests are made by Evergreen's native HTTP client, so page CORS restrictions do not apply. Redirect following is disabled to prevent an allowed endpoint redirecting the request to an undeclared host.

Example:

```javascript
// ==UserScript==
// @name         Local contacts helper
// @match        https://web.whatsapp.com/*
// @run-at       document-start
// @grant        GM_xmlhttpRequest
// @connect      127.0.0.1
// @connect      localhost
// ==/UserScript==
```

Security note: `@connect` is enforced by the native host, and the native bridge uses a per-tab token. Only install scripts you trust. Script management UI, `GM_setValue`/`GM_getValue`, cross-frame injection, and the wider Tampermonkey API are not implemented yet.
