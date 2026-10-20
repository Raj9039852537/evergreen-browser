//! Lightweight Tampermonkey-style userscript loader and GM_xmlhttpRequest bridge.
//! Scripts are user-installed local files; only @match pages execute them and
//! GM_xmlhttpRequest destinations are checked against declared @connect hosts.
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct UserScript {
    pub path: PathBuf,
    pub name: String,
    pub source: String,
    pub matches: Vec<String>,
    pub grants: Vec<String>,
    pub connects: Vec<String>,
    pub run_at: String,
    pub enabled: bool,
}

pub fn load_from_directory(dir: &Path) -> Vec<UserScript> {
    let mut scripts = Vec::new();
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("[userscripts] Cannot create {}: {e}", dir.display());
        return scripts;
    }
    let Ok(entries) = fs::read_dir(dir) else { return scripts; };
    for entry in entries.flatten() {
        let path = entry.path();
        let ext_ok = path.extension().and_then(|x| x.to_str())
            .map(|x| x.eq_ignore_ascii_case("js")).unwrap_or(false);
        if !ext_ok || !path.is_file() { continue; }
        let Ok(source) = fs::read_to_string(&path) else { continue; };
        if !source.contains("==UserScript==") { continue; }
        let mut script = UserScript {
            name: path.file_name().and_then(|x| x.to_str()).unwrap_or("Userscript").to_string(),
            path, source, matches: Vec::new(), grants: Vec::new(), connects: Vec::new(),
            run_at: "document-end".into(), enabled: true,
        };
        for line in script.source.lines() {
            let line = line.trim().trim_start_matches("//").trim();
            let mut parts = line.splitn(2, char::is_whitespace);
            let key = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("").trim();
            match key {
                "@name" if !value.is_empty() => script.name = value.to_string(),
                "@match" | "@include" if !value.is_empty() => script.matches.push(value.to_string()),
                "@grant" if !value.is_empty() => script.grants.push(value.to_string()),
                "@connect" if !value.is_empty() => script.connects.push(value.to_ascii_lowercase()),
                "@run-at" if !value.is_empty() => script.run_at = value.to_string(),
                "@exclude" | "@exclude-match" => {},
                _ => {},
            }
        }
        if !script.matches.is_empty() || !script.grants.is_empty() {
            scripts.push(script);
        }
    }
    scripts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    eprintln!("[userscripts] Loaded {} script(s) from {}", scripts.len(), dir.display());
    scripts
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    let p = pattern.as_bytes();
    let v = value.as_bytes();
    let (mut i, mut j, mut star, mut mark) = (0usize, 0usize, None, 0usize);
    while j < v.len() {
        if i < p.len() && (p[i] == v[j] || p[i] == b'*') {
            if p[i] == b'*' { star = Some(i); mark = j; i += 1; }
            else { i += 1; j += 1; }
        } else if let Some(s) = star {
            i = s + 1; mark += 1; j = mark;
        } else { return false; }
    }
    while i < p.len() && p[i] == b'*' { i += 1; }
    i == p.len()
}

fn script_matches(script: &UserScript, url: &str) -> bool {
    if script.matches.is_empty() { return false; }
    script.matches.iter().any(|p| {
        if p == "<all_urls>" { return url.starts_with("http://") || url.starts_with("https://"); }
        wildcard_match(p, url)
    })
}

pub fn allowed_connect_hosts(scripts: &[UserScript], page_url: &str) -> Vec<String> {
    let mut hosts = Vec::new();
    for script in scripts.iter().filter(|s| s.enabled && script_matches(s, page_url)) {
        if script.grants.iter().any(|g| g == "GM_xmlhttpRequest" || g == "GM.xmlHttpRequest") {
            hosts.extend(script.connects.iter().cloned());
        }
    }
    hosts.sort(); hosts.dedup(); hosts
}

pub fn is_connect_allowed(url: &str, allowed: &[String]) -> bool {
    let Some((scheme, rest)) = url.split_once("://") else { return false; };
    if scheme != "http" && scheme != "https" { return false; }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or("");
    let host = if host_port.starts_with('[') {
        host_port.split(']').next().unwrap_or("").trim_start_matches('[').to_ascii_lowercase()
    } else { host_port.split(':').next().unwrap_or("").to_ascii_lowercase() };
    allowed.iter().any(|entry| {
        let entry = entry.trim().to_ascii_lowercase();
        if entry == "*" { return true; }
        if let Some(suffix) = entry.strip_prefix("*.") {
            host == suffix || host.ends_with(&format!(".{suffix}"))
        } else { host == entry }
    })
}

pub fn build_initialization_script(scripts: &[UserScript], token: &str) -> String {
    let active: Vec<&UserScript> = scripts.iter().filter(|s| s.enabled).collect();
    if active.is_empty() { return String::new(); }
    let specs: Vec<Value> = active.iter().map(|s| json!({
        "name": s.name, "source": s.source, "matches": s.matches,
        "grants": s.grants, "runAt": s.run_at
    })).collect();
    let specs = serde_json::to_string(&specs).unwrap_or_else(|_| "[]".into());
    let token = serde_json::to_string(token).unwrap_or_else(|_| "\"\"".into());
    format!(r#"(() => {{
const __egScripts = {specs};
const __egToken = {token};
const __egMatches = (pattern, url) => {{
  if (pattern === '<all_urls>') return /^https?:/.test(url);
  const escaped = pattern.replace(/[.+?^${{}}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*');
  try {{ return new RegExp('^' + escaped + '$').test(url); }} catch (_) {{ return false; }}
}};
const __egPending = new Map();
let __egNextId = 1;
window.__egUserscriptHttpResponse = (id, packed) => {{
  const pending = __egPending.get(String(id)); if (!pending) return;
  __egPending.delete(String(id));
  let r; try {{ r = JSON.parse(packed); }} catch (_) {{ r = {{status:0,statusText:'Invalid native response',responseText:'',responseHeaders:''}}; }}
  const response = Object.assign({{}}, r, {{readyState:4, finalUrl:r.finalUrl || pending.url}});
  try {{ if (pending.onreadystatechange) pending.onreadystatechange(response); }} catch (_) {{}}
  try {{ if (r.status > 0 && pending.onload) pending.onload(response); else if (pending.onerror) pending.onerror(response); }} catch (_) {{}}
  try {{ if (pending.onloadend) pending.onloadend(response); }} catch (_) {{}}
}};
const __egGM = {{
  GM_xmlhttpRequest: (details = {{}}) => {{
    const url = String(details.url || '');
    const id = String(__egNextId++);
    const pending = Object.assign({{}}, details, {{url}});
    __egPending.set(id, pending);
    try {{
      if (!window.ipc || !window.ipc.postMessage) throw new Error('Native userscript bridge unavailable');
      window.ipc.postMessage(JSON.stringify({{action:'UserscriptHttpRequest',payload:{{request_id:id,token:__egToken,url,method:String(details.method || 'GET').toUpperCase(),headers:details.headers || {{}},data:typeof details.data === 'string' ? details.data : (details.data == null ? null : String(details.data))}}}}));
    }} catch (e) {{
      __egPending.delete(id); const response = {{status:0,statusText:String(e),responseText:'',readyState:4}};
      try {{ if (details.onerror) details.onerror(response); }} catch (_) {{}}
    }}
    return {{ abort: () => {{ __egPending.delete(id); }} }};
  }}
}};
for (const item of __egScripts) {{
  if (!item.matches.some(p => __egMatches(p, location.href))) continue;
  const grants = new Set(item.grants || []);
  const sandbox = {{ console, setTimeout, clearTimeout, setInterval, clearInterval, URL, URLSearchParams }};
  if (grants.has('GM_xmlhttpRequest') || grants.has('GM.xmlHttpRequest')) sandbox.GM_xmlhttpRequest = __egGM.GM_xmlhttpRequest;
  if (grants.has('GM.xmlHttpRequest')) sandbox.GM = {{xmlHttpRequest: __egGM.GM_xmlhttpRequest}};
  try {{ (new Function('window','document','location','GM_xmlhttpRequest','GM','unsafeWindow', item.source + '\\n//# sourceURL=userscript://' + encodeURIComponent(item.name)))(window,document,location,sandbox.GM_xmlhttpRequest,sandbox.GM,window); }}
  catch (e) {{ console.error('[Evergreen userscript]', item.name, e); }}
}}
}})();"#)
}

pub fn perform_http_request(url: &str, method: &str, headers: &Value, body: Option<&str>) -> String {
    let result = (|| -> Result<Value, String> {
        let client = reqwest::blocking::Client::builder().redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(60)).build().map_err(|e| e.to_string())?;
        let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?;
        let mut request = client.request(method, url);
        if let Some(map) = headers.as_object() {
            for (key, value) in map {
                let Some(value) = value.as_str() else { continue; };
                let name = reqwest::header::HeaderName::from_bytes(key.as_bytes()).map_err(|e| e.to_string())?;
                let value = reqwest::header::HeaderValue::from_str(value).map_err(|e| e.to_string())?;
                request = request.header(name, value);
            }
        }
        if let Some(body) = body { request = request.body(body.to_string()); }
        let response = request.send().map_err(|e| e.to_string())?;
        let status = response.status();
        let final_url = response.url().to_string();
        let headers_text = response.headers().iter().map(|(k,v)| format!("{}: {}\r\n", k.as_str(), v.to_str().unwrap_or(""))).collect::<String>();
        let text = response.text().map_err(|e| e.to_string())?;
        Ok(json!({"status":status.as_u16(),"statusText":status.canonical_reason().unwrap_or(""),"responseText":text,"responseHeaders":headers_text,"finalUrl":final_url,"readyState":4}))
    })();
    match result { Ok(v) => v.to_string(), Err(e) => json!({"status":0,"statusText":e,"responseText":"","responseHeaders":"","readyState":4}).to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_host_policy_matches_exact_hosts_and_subdomains() {
        let allowed = vec!["127.0.0.1".to_string(), "localhost".to_string(), "*.example.org".to_string()];
        assert!(is_connect_allowed("http://127.0.0.1:5001/contacts/api/contacts", &allowed));
        assert!(is_connect_allowed("http://localhost:5001/contacts/", &allowed));
        assert!(is_connect_allowed("https://api.example.org/data", &allowed));
        assert!(!is_connect_allowed("https://example.com/", &allowed));
        assert!(!is_connect_allowed("file:///etc/passwd", &allowed));
    }

    #[test]
    fn wildcard_match_supports_userscript_match_patterns() {
        assert!(wildcard_match("https://web.whatsapp.com/*", "https://web.whatsapp.com/"));
        assert!(wildcard_match("https://web.whatsapp.com/*", "https://web.whatsapp.com/chat/abc"));
        assert!(!wildcard_match("https://web.whatsapp.com/*", "https://example.com/"));
    }
}
