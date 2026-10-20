# Session restoration

Evergreen stores a small `session.json` beside `settings.json` in the resolved Evergreen data directory. The version-1 snapshot contains only tab URLs, tab order, and the active tab index. It does not serialize cookies, site storage, cache, WebView profiles, or any other browsing data.

On startup, an explicit URL passed on the command line takes precedence over the saved session. Otherwise, saved tabs are reopened in their prior order and the prior active tab is selected. A malformed/unsupported snapshot is ignored. Session writes are triggered by tab creation/closure, URL navigation/page navigation, tab reorder, and tab selection from the browser UI. Closing the final tab removes the saved session.

The existing per-site persistence allowlist and WebView2 partition selection are unchanged. This session file is intentionally separate from WebView2's ephemeral/persistent site data.
