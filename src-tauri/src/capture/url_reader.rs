use reqwest::Client;
use std::time::Duration;

const JINA_READER_BASE: &str = "https://r.jina.ai/";
const MAX_CONTENT_LENGTH: usize = 50_000; // ~50KB
const FETCH_TIMEOUT_SECS: u64 = 15;
const MIN_CONTENT_LENGTH: usize = 20;

const BROWSER_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct UrlReadResult {
    pub content: String,
    pub title: Option<String>,
}

pub struct UrlReader {
    http_client: Client,
}

impl UrlReader {
    pub fn new() -> Self {
        let http_client = match Client::builder()
            .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                log::error!("Failed to build HTTP client: {}, using default", e);
                Client::new()
            }
        };
        UrlReader { http_client }
    }

    /// Smart fetch: pick the best method based on URL domain.
    /// Order: platform-specific → Jina Reader → direct HTML (fallback).
    pub async fn fetch_content(&self, url: &str) -> Result<UrlReadResult, String> {
        let clean_url = url.trim();
        if clean_url.is_empty() {
            return Err("Empty URL".to_string());
        }

        // ── Platform-specific readers (fastest, most reliable) ──

        // WeChat: Jina is blocked, must use direct HTML
        if clean_url.contains("mp.weixin.qq.com") {
            log::info!("[WeChat] 直接抓取: {}", clean_url);
            return self.fetch_wechat(clean_url).await;
        }

        // X/Twitter: Jina only gets login wall, use fxtwitter API
        if clean_url.contains("x.com/") || clean_url.contains("twitter.com/") {
            log::info!("[Twitter] fxtwitter API: {}", clean_url);
            return self.fetch_twitter(clean_url).await;
        }

        // GitHub: use API for repos, Jina for others
        if clean_url.contains("github.com/") {
            if let Some(result) = self.try_fetch_github(clean_url).await {
                return result;
            }
            // Fall through to Jina for non-repo GitHub pages
        }

        // Reddit: use JSON API (Jina often rate-limited by Reddit)
        if clean_url.contains("reddit.com/") {
            log::info!("[Reddit] JSON API: {}", clean_url);
            match self.fetch_reddit(clean_url).await {
                Ok(r) => return Ok(r),
                Err(e) => log::warn!("[Reddit] API failed, trying Jina: {}", e),
            }
        }

        // ── General: Jina Reader → direct HTML fallback ──
        log::info!("[Jina] 通用读取: {}", clean_url);
        match self.fetch_via_jina(clean_url).await {
            Ok(r) => Ok(r),
            Err(jina_err) => {
                log::warn!("[Jina] 失败 ({}), 尝试直接抓取", jina_err);
                // Fallback: direct HTML fetch + tag stripping
                self.fetch_direct_html(clean_url).await
                    .map_err(|html_err| {
                        format!("Jina: {} | Direct: {}", jina_err, html_err)
                    })
            }
        }
    }

    // ─── WeChat ────────────────────────────────────────────────────

    async fn fetch_wechat(&self, url: &str) -> Result<UrlReadResult, String> {
        let html = self.get_html(url).await?;
        let title = extract_wechat_title(&html);

        // Try js_content div first (standard articles)
        let content = extract_wechat_content(&html);

        if content.len() >= MIN_CONTENT_LENGTH {
            let markdown = format_with_title(&title, &truncate_content(content));
            log::info!("[WeChat] 成功 (js_content): {} chars, title={:?}", markdown.len(), title);
            return Ok(UrlReadResult { content: markdown, title });
        }

        // Fallback: og:description (for appmsg_type=9 short articles, shares, etc.)
        log::info!("[WeChat] js_content too short, trying og:description fallback");
        if let Some(desc) = extract_og_description(&html) {
            if desc.len() >= MIN_CONTENT_LENGTH {
                let decoded = desc.replace("\\x0a", "\n").replace("\\x26amp;amp;", "&");
                let markdown = format_with_title(&title, &truncate_content(decoded));
                log::info!("[WeChat] 成功 (og:description): {} chars, title={:?}", markdown.len(), title);
                return Ok(UrlReadResult { content: markdown, title });
            }
        }

        Err(format!("WeChat content too short ({} chars)", content.len()))
    }

    // ─── X/Twitter ─────────────────────────────────────────────────

    async fn fetch_twitter(&self, url: &str) -> Result<UrlReadResult, String> {
        let (user, tweet_id) = parse_twitter_url(url)
            .ok_or_else(|| format!("Cannot parse Twitter URL: {}", url))?;

        let api_url = format!("https://api.fxtwitter.com/{}/status/{}", user, tweet_id);
        let json: serde_json::Value = self.get_json(&api_url).await?;

        let tweet = json.get("tweet").ok_or("fxtwitter: no tweet")?;
        let author_name = tweet.pointer("/author/name").and_then(|v| v.as_str()).unwrap_or("");
        let author_handle = tweet.pointer("/author/screen_name").and_then(|v| v.as_str()).unwrap_or("");

        let (title, body) = if let Some(article) = tweet.get("article") {
            let t = article.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
            (t, extract_twitter_article_content(article))
        } else {
            let text = tweet.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (None, text)
        };

        if body.len() < MIN_CONTENT_LENGTH {
            return Err("Tweet content too short".to_string());
        }

        let content = truncate_content(body);
        let markdown = if let Some(ref t) = title {
            format!("# {}\n\n> @{} ({})\n\n{}", t, author_handle, author_name, content)
        } else {
            format!("> @{} ({})\n\n{}", author_handle, author_name, content)
        };

        log::info!("[Twitter] 成功: {} chars", markdown.len());
        Ok(UrlReadResult {
            content: markdown,
            title: title.or_else(|| Some(format!("@{}: {}…", author_handle, content.chars().take(50).collect::<String>()))),
        })
    }

    // ─── GitHub ────────────────────────────────────────────────────

    /// Try GitHub API for repo URLs (owner/repo). Returns None for non-repo URLs.
    async fn try_fetch_github(&self, url: &str) -> Option<Result<UrlReadResult, String>> {
        let (owner, repo) = parse_github_repo_url(url)?;
        log::info!("[GitHub] API 读取: {}/{}", owner, repo);

        Some(self.fetch_github_repo(&owner, &repo).await)
    }

    async fn fetch_github_repo(&self, owner: &str, repo: &str) -> Result<UrlReadResult, String> {
        // 1. Get repo info
        let repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let repo_json: serde_json::Value = self
            .http_client
            .get(&repo_url)
            .header("User-Agent", "xiaoyun/0.1")
            .header("Accept", "application/vnd.github.v3+json")
            .send().await.map_err(|e| format!("GitHub API: {}", e))?
            .json().await.map_err(|e| format!("GitHub JSON: {}", e))?;

        let description = repo_json.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let stars = repo_json.get("stargazers_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let language = repo_json.get("language").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let repo_name = repo_json.get("full_name").and_then(|v| v.as_str()).unwrap_or("");

        // 2. Try to get README
        let readme_url = format!("https://api.github.com/repos/{}/{}/readme", owner, repo);
        let readme_content = match self.http_client
            .get(&readme_url)
            .header("User-Agent", "xiaoyun/0.1")
            .header("Accept", "application/vnd.github.v3+json")
            .send().await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    // README is base64-encoded
                    json.get("content")
                        .and_then(|v| v.as_str())
                        .and_then(|b64| {
                            let clean = b64.replace('\n', "");
                            base64_decode(&clean)
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        };

        // 3. Format
        let header = format!(
            "# {}\n\n{}\n\n⭐ {} stars · Language: {}\n",
            repo_name, description, stars, language
        );

        let content = if readme_content.is_empty() {
            header
        } else {
            let readme_trimmed = truncate_content(readme_content);
            format!("{}\n---\n\n{}", header, readme_trimmed)
        };

        let title = Some(format!("{} — {}", repo_name, description));
        log::info!("[GitHub] 成功: {} chars", content.len());
        Ok(UrlReadResult { content, title })
    }

    // ─── Reddit ────────────────────────────────────────────────────

    async fn fetch_reddit(&self, url: &str) -> Result<UrlReadResult, String> {
        // Append .json to Reddit URL to get JSON response
        let clean = url.split('?').next().unwrap_or(url).trim_end_matches('/');
        let json_url = format!("{}.json", clean);

        let json: serde_json::Value = self
            .http_client
            .get(&json_url)
            .header("User-Agent", "xiaoyun/0.1")
            .send().await.map_err(|e| format!("Reddit: {}", e))?
            .json().await.map_err(|e| format!("Reddit JSON: {}", e))?;

        // Reddit returns an array: [post_listing, comments_listing]
        let post_data = json.as_array()
            .and_then(|arr| arr.first())
            .and_then(|listing| listing.pointer("/data/children/0/data"))
            .ok_or("Reddit: cannot find post data")?;

        let title = post_data.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let selftext = post_data.get("selftext").and_then(|v| v.as_str()).unwrap_or("");
        let subreddit = post_data.get("subreddit").and_then(|v| v.as_str()).unwrap_or("");
        let author = post_data.get("author").and_then(|v| v.as_str()).unwrap_or("");
        let score = post_data.get("score").and_then(|v| v.as_i64()).unwrap_or(0);

        let body = if selftext.is_empty() {
            // Link post — might have a linked URL
            let linked_url = post_data.get("url").and_then(|v| v.as_str()).unwrap_or("");
            format!("Link: {}", linked_url)
        } else {
            selftext.to_string()
        };

        let markdown = format!(
            "# {}\n\n> r/{} · u/{} · {} points\n\n{}",
            title, subreddit, author, score, truncate_content(body)
        );

        log::info!("[Reddit] 成功: {} chars", markdown.len());
        Ok(UrlReadResult {
            content: markdown,
            title: if title.is_empty() { None } else { Some(title) },
        })
    }

    // ─── Jina Reader ───────────────────────────────────────────────

    async fn fetch_via_jina(&self, url: &str) -> Result<UrlReadResult, String> {
        let jina_url = format!("{}{}", JINA_READER_BASE, url);

        let response = self.http_client
            .get(&jina_url)
            .header("X-Return-Format", "markdown")
            .send().await
            .map_err(|e| format!("Jina request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Jina status: {}", response.status()));
        }

        let body = response.text().await
            .map_err(|e| format!("Jina read failed: {}", e))?;

        if body.trim().len() < MIN_CONTENT_LENGTH {
            return Err("Content too short".to_string());
        }

        let content = truncate_content(body);
        let title = extract_markdown_title(&content);
        Ok(UrlReadResult { content, title })
    }

    // ─── Direct HTML Fallback ──────────────────────────────────────

    /// Last resort: fetch raw HTML and strip tags.
    /// Works for most server-rendered pages, fails for SPAs.
    async fn fetch_direct_html(&self, url: &str) -> Result<UrlReadResult, String> {
        log::info!("[Direct] 直接抓取 HTML: {}", url);
        let html = self.get_html(url).await?;

        let title = extract_html_title(&html);
        let content = strip_html_to_text(&html);

        if content.len() < MIN_CONTENT_LENGTH {
            return Err(format!("Direct HTML: content too short ({} chars)", content.len()));
        }

        let markdown = format_with_title(&title, &truncate_content(content));
        log::info!("[Direct] 成功: {} chars, title={:?}", markdown.len(), title);
        Ok(UrlReadResult { content: markdown, title })
    }

    // ─── HTTP helpers ──────────────────────────────────────────────

    async fn get_html(&self, url: &str) -> Result<String, String> {
        self.http_client
            .get(url)
            .header("User-Agent", BROWSER_UA)
            .send().await.map_err(|e| format!("HTTP request failed: {}", e))?
            .text().await.map_err(|e| format!("HTTP read failed: {}", e))
    }

    async fn get_json(&self, url: &str) -> Result<serde_json::Value, String> {
        self.http_client
            .get(url)
            .header("User-Agent", "xiaoyun/0.1")
            .send().await.map_err(|e| format!("HTTP request failed: {}", e))?
            .json().await.map_err(|e| format!("JSON parse failed: {}", e))
    }
}

// ═══════════════════════════════════════════════════════════════════
// Helper functions
// ═══════════════════════════════════════════════════════════════════

fn truncate_content(content: String) -> String {
    if content.len() > MAX_CONTENT_LENGTH {
        let truncated: String = content.chars().take(MAX_CONTENT_LENGTH).collect();
        format!("{}...\n\n[内容已截断]", truncated)
    } else {
        content
    }
}

fn format_with_title(title: &Option<String>, content: &str) -> String {
    if let Some(t) = title {
        format!("# {}\n\n{}", t, content)
    } else {
        content.to_string()
    }
}

fn extract_markdown_title(markdown: &str) -> Option<String> {
    for line in markdown.lines().take(10) {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return Some(trimmed.trim_start_matches('#').trim().to_string());
        }
    }
    None
}

// ─── WeChat helpers ────────────────────────────────────────────────

fn extract_wechat_title(html: &str) -> Option<String> {
    // msg_title = '...' (single quotes)
    if let Some(start) = html.find("msg_title = '") {
        let rest = &html[start + 13..];
        if let Some(end) = rest.find('\'') {
            let title = rest[..end].trim().to_string();
            if !title.is_empty() { return Some(html_decode(&title)); }
        }
    }
    // msg_title = "..." (double quotes)
    if let Some(start) = html.find("msg_title = \"") {
        let rest = &html[start + 13..];
        if let Some(end) = rest.find('"') {
            let title = rest[..end].trim().to_string();
            if !title.is_empty() { return Some(html_decode(&title)); }
        }
    }
    extract_og_title(html)
}

fn extract_wechat_content(html: &str) -> String {
    let marker = "id=\"js_content\"";
    let start_idx = match html.find(marker) {
        Some(idx) => idx,
        None => return String::new(),
    };

    let rest = &html[start_idx..];
    let content_start = match rest.find('>') {
        Some(idx) => start_idx + idx + 1,
        None => return String::new(),
    };

    let content_html = &html[content_start..];
    let mut result = String::new();
    let mut in_tag = false;
    let mut div_depth: i32 = 1;
    let mut chars = content_html.chars().peekable();

    while let Some(ch) = chars.next() {
        if result.len() > MAX_CONTENT_LENGTH { break; }
        match ch {
            '<' => {
                in_tag = true;
                let upcoming: String = chars.clone().take(10).collect();
                if upcoming.starts_with("div") || upcoming.starts_with("section") {
                    div_depth += 1;
                } else if upcoming.starts_with("/div") || upcoming.starts_with("/section") {
                    div_depth -= 1;
                    if div_depth <= 0 { break; }
                }
                if upcoming.starts_with("br") || upcoming.starts_with("/p")
                    || upcoming.starts_with("/div") || upcoming.starts_with("/section") {
                    if !result.ends_with('\n') { result.push('\n'); }
                }
            }
            '>' => { in_tag = false; }
            _ => { if !in_tag { result.push(ch); } }
        }
    }

    let decoded = html_decode(&result);
    decoded.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

// ─── Twitter helpers ───────────────────────────────────────────────

fn parse_twitter_url(url: &str) -> Option<(String, String)> {
    let clean = url.trim().trim_end_matches('/').split('?').next().unwrap_or(url);
    let parts: Vec<&str> = clean.split('/').collect();
    for i in 0..parts.len() {
        if parts[i] == "status" && i > 0 && i + 1 < parts.len() {
            let user = parts[i - 1].to_string();
            let id = parts[i + 1].to_string();
            if !user.is_empty() && !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                return Some((user, id));
            }
        }
    }
    None
}

fn extract_twitter_article_content(article: &serde_json::Value) -> String {
    let mut lines = Vec::new();
    if let Some(blocks) = article.pointer("/content/blocks").and_then(|v| v.as_array()) {
        for block in blocks {
            let text = block.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if text.is_empty() { continue; }
            let btype = block.get("type").and_then(|v| v.as_str()).unwrap_or("unstyled");
            match btype {
                "header-one" => lines.push(format!("## {}", text)),
                "header-two" => lines.push(format!("### {}", text)),
                "header-three" => lines.push(format!("#### {}", text)),
                "blockquote" => lines.push(format!("> {}", text)),
                "ordered-list-item" | "unordered-list-item" => lines.push(format!("- {}", text)),
                "atomic" => {}
                _ => lines.push(text.to_string()),
            }
        }
    }
    lines.join("\n\n")
}

// ─── GitHub helpers ────────────────────────────────────────────────

/// Parse GitHub repo URL: https://github.com/owner/repo[/...]
fn parse_github_repo_url(url: &str) -> Option<(String, String)> {
    let clean = url.trim().trim_end_matches('/').split('?').next().unwrap_or(url);
    let parts: Vec<&str> = clean.split('/').collect();
    // Find "github.com" and get the next two segments
    for i in 0..parts.len() {
        if parts[i] == "github.com" && i + 2 < parts.len() {
            let owner = parts[i + 1].to_string();
            let repo = parts[i + 2].to_string();
            if !owner.is_empty() && !repo.is_empty()
                && owner != "." && repo != "."
                && !owner.starts_with('-')
            {
                return Some((owner, repo));
            }
        }
    }
    None
}

fn base64_decode(input: &str) -> Option<String> {
    // Simple base64 decoder for GitHub API responses
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bytes = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &b in input.as_bytes() {
        if b == b'=' || b == b'\n' || b == b'\r' || b == b' ' { continue; }
        let val = table.iter().position(|&c| c == b)? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    String::from_utf8(bytes).ok()
}

// ─── Generic HTML helpers ──────────────────────────────────────────

fn extract_og_description(html: &str) -> Option<String> {
    if let Some(start) = html.find("og:description") {
        let rest = &html[start..];
        if let Some(c_start) = rest.find("content=\"") {
            let c_rest = &rest[c_start + 9..];
            if let Some(end) = c_rest.find('"') {
                let desc = c_rest[..end].trim().to_string();
                if !desc.is_empty() { return Some(html_decode(&desc)); }
            }
        }
    }
    None
}

fn extract_og_title(html: &str) -> Option<String> {
    if let Some(start) = html.find("og:title") {
        let rest = &html[start..];
        if let Some(c_start) = rest.find("content=\"") {
            let c_rest = &rest[c_start + 9..];
            if let Some(end) = c_rest.find('"') {
                let title = c_rest[..end].trim().to_string();
                if !title.is_empty() { return Some(html_decode(&title)); }
            }
        }
    }
    None
}

fn extract_html_title(html: &str) -> Option<String> {
    // Try og:title first
    if let Some(t) = extract_og_title(html) {
        return Some(t);
    }
    // Try <title>...</title>
    if let Some(start) = html.find("<title>") {
        let rest = &html[start + 7..];
        if let Some(end) = rest.find("</title>") {
            let title = rest[..end].trim().to_string();
            if !title.is_empty() { return Some(html_decode(&title)); }
        }
    }
    None
}

/// Strip all HTML tags and extract readable text from <body> or <article>.
fn strip_html_to_text(html: &str) -> String {
    // Try to find <article> or <main> or <body>
    let start_markers = ["<article", "<main", "<body"];
    let start_idx = start_markers.iter()
        .filter_map(|m| html.find(m))
        .min()
        .unwrap_or(0);

    let content_html = &html[start_idx..];
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    let bytes = content_html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len && result.len() < MAX_CONTENT_LENGTH {
        let b = bytes[i];
        if b == b'<' {
            in_tag = true;
            // Check for <script or <style
            let upcoming = &content_html[i..std::cmp::min(i + 20, len)].to_lowercase();
            if upcoming.starts_with("<script") { in_script = true; }
            else if upcoming.starts_with("</script") { in_script = false; }
            else if upcoming.starts_with("<style") { in_style = true; }
            else if upcoming.starts_with("</style") { in_style = false; }
            // Block-level tags → newline
            if upcoming.starts_with("<br") || upcoming.starts_with("</p")
                || upcoming.starts_with("</div") || upcoming.starts_with("</h")
                || upcoming.starts_with("</li") || upcoming.starts_with("</tr")
            {
                if !result.ends_with('\n') { result.push('\n'); }
            }
            i += 1;
            continue;
        }
        if b == b'>' {
            in_tag = false;
            i += 1;
            continue;
        }
        if !in_tag && !in_script && !in_style {
            result.push(b as char);
        }
        i += 1;
    }

    let decoded = html_decode(&result);
    let lines: Vec<&str> = decoded.lines()
        .map(|l| l.trim())
        .filter(|l| l.len() > 1) // skip single-char noise
        .collect();
    lines.join("\n")
}

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("&#x3D;", "=")
}
