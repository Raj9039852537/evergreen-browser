# Userscript support (initial)

Evergreen Browser can load local JavaScript userscripts from the `userscripts` directory inside the browser's data directory (the same directory that holds `settings.json` and `session.json`). The app creates this folder on startup. Add a `.user.js` file and restart Evergreen.

## Supported metadata

- `@name`
- `@match` with `*` wildcard matching
- `@include` with the same wildcard matching
- `@grant GM_xmlhttpRequest` or `@grant GM.xmlHttpRequest`
- `@connect` host declarations, including exact hosts and `*.example.com`
- `@run-at` is recognized, but all scripts currently run at document start

## GM_xmlhttpRequest

The browser makes the request natively rather than using page `fetch`/XHR, bypassing browser-page CORS enforcement. The native side checks the requesting tab's URL against userscript `@match` rules, validates a per-tab bridge token, and enforces the installed script's `@connect` allowlist. Native redirects are disabled so an allowed URL cannot redirect to an undeclared host.

Current response support includes `status`, `statusText`, `responseText`, `responseHeaders`, `finalUrl`, and the `onload`, `onerror`, `onreadystatechange`, and `onloadend` callbacks. This is an initial compatibility layer, not a complete Tampermonkey implementation. Script-management UI, update checks, `GM_setValue`, `GM_getValue`, `GM_download`, `GM_openInTab`, `@require`, and isolated-world sandboxing are not yet included.

## Example for local WhatsApp contacts API

```javascript
// ==UserScript==
// @name         WhatsApp Contacts Name Replacer
// @namespace    local.whatsapp.contacts.replacer
// @version      3.6.0
// @description  Replace WhatsApp Web phone numbers with names from a local contacts API
// @match        https://web.whatsapp.com/*
// @run-at       document-start
// @grant        GM_xmlhttpRequest
// @connect      127.0.0.1
// @connect      localhost
// ==/UserScript==
```
