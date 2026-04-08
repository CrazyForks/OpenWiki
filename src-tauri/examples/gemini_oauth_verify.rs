//! Google Gemini (Antigravity) OAuth 验证脚本
//!
//! 验证目标：用 Antigravity 的 OAuth 凭证登录 Google，获取 projectId，
//! 然后调用 cloudcode-pa API 发消息给 Gemini，确认能走通。
//!
//! 运行方式：cd src-tauri && cargo run --example gemini_oauth_verify

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

const CLIENT_ID: &str = "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
const CLIENT_SECRET: &str = "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";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const REDIRECT_URI: &str = "http://localhost:51121/oauth-callback";
const CALLBACK_PORT: u16 = 51121;
const SCOPES: &str = "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs";

const ENDPOINT_PROD: &str = "https://cloudcode-pa.googleapis.com";
const ENDPOINT_DAILY: &str = "https://daily-cloudcode-pa.sandbox.googleapis.com";

fn generate_pkce() -> (String, String) {
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen()).collect();
    let verifier = URL_SAFE_NO_PAD.encode(&random_bytes);
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                format!("{}", b as char)
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Google Gemini (Antigravity) OAuth 验证 ===\n");

    // Step 1: Generate PKCE
    let (verifier, challenge) = generate_pkce();
    println!("[1/5] PKCE 已生成");

    // Step 2: Start local callback server
    let listener = match TcpListener::bind(format!("127.0.0.1:{}", CALLBACK_PORT)).await {
        Ok(l) => {
            println!("[2/5] 回调服务器已启动: {}", REDIRECT_URI);
            l
        }
        Err(e) => {
            eprintln!("❌ 端口 {} 被占用: {}", CALLBACK_PORT, e);
            return Err(format!("Port {} in use", CALLBACK_PORT).into());
        }
    };

    // Step 3: Build auth URL and open browser
    // state encodes verifier + projectId as base64url JSON
    let state_json = serde_json::json!({"verifier": verifier, "projectId": ""});
    let state = URL_SAFE_NO_PAD.encode(state_json.to_string().as_bytes());

    let auth_url = format!(
        "{}?client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}&access_type=offline&prompt=consent",
        AUTH_URL,
        url_encode(CLIENT_ID),
        url_encode(REDIRECT_URI),
        url_encode(SCOPES),
        challenge,
        url_encode(&state),
    );

    println!("\n[3/5] 请在浏览器中登录 Google：\n");
    println!("  {}\n", auth_url);

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&auth_url).spawn();
        println!("  (已尝试自动打开浏览器)");
    }

    println!("\n  等待授权回调...\n");

    // Wait for callback (180s timeout)
    let (code, _returned_state) = wait_for_callback(&listener).await?;
    println!("[3/5] ✓ 收到授权码");

    // Step 4: Exchange code for token
    println!("\n[4/5] 正在换取 token...");
    let client = reqwest::Client::new();
    let token_resp = client
        .post(TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded;charset=UTF-8")
        .header("Accept", "*/*")
        .header("User-Agent", "google-api-nodejs-client/9.15.1")
        .body(format!(
            "client_id={}&client_secret={}&code={}&grant_type=authorization_code&redirect_uri={}&code_verifier={}",
            url_encode(CLIENT_ID),
            url_encode(CLIENT_SECRET),
            url_encode(&code),
            url_encode(REDIRECT_URI),
            url_encode(&verifier),
        ))
        .send()
        .await?;

    let token_status = token_resp.status();
    let token_body = token_resp.text().await?;

    if !token_status.is_success() {
        eprintln!("❌ Token 交换失败 (HTTP {}):", token_status);
        eprintln!("  {}", token_body);
        return Err("Token exchange failed".into());
    }

    let token_data: serde_json::Value = serde_json::from_str(&token_body)?;
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or("No access_token")?;
    let refresh_token = token_data.get("refresh_token").and_then(|v| v.as_str());
    let expires_in = token_data.get("expires_in").and_then(|v| v.as_i64());

    println!("  ✓ Token 交换成功！");
    println!("  access_token 长度: {}", access_token.len());
    println!("  refresh_token: {}", if refresh_token.is_some() { "有" } else { "无" });
    println!("  expires_in: {}秒", expires_in.unwrap_or(0));

    // Get user email
    let user_resp = client
        .get("https://www.googleapis.com/oauth2/v1/userinfo?alt=json")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;
    if user_resp.status().is_success() {
        let user_data: serde_json::Value = user_resp.json().await?;
        if let Some(email) = user_data.get("email").and_then(|v| v.as_str()) {
            println!("  用户邮箱: {}", email);
        }
    }

    // Step 4b: Get projectId via loadCodeAssist
    println!("\n  获取 projectId...");
    let mut project_id = String::new();

    for endpoint in &[ENDPOINT_PROD, ENDPOINT_DAILY] {
        let url = format!("{}/v1internal:loadCodeAssist", endpoint);
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .header("Client-Metadata", r#"{"ideType":"ANTIGRAVITY","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#)
            .json(&serde_json::json!({
                "metadata": {
                    "ideType": "ANTIGRAVITY",
                    "platform": "PLATFORM_UNSPECIFIED",
                    "pluginType": "GEMINI"
                }
            }))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let data: serde_json::Value = r.json().await?;
                if let Some(pid) = data.get("cloudaicompanionProject").and_then(|v| v.as_str()) {
                    project_id = pid.to_string();
                    println!("  ✓ projectId: {} (from {})", project_id, endpoint);
                    break;
                } else if let Some(pid) = data.get("cloudaicompanionProject")
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str()) {
                    project_id = pid.to_string();
                    println!("  ✓ projectId: {} (from {})", project_id, endpoint);
                    break;
                } else {
                    println!("  端点 {} 响应: {}", endpoint, data);
                }
            }
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                println!("  端点 {} 失败 (HTTP {}): {}", endpoint, status, &body[..body.len().min(200)]);
            }
            Err(e) => {
                println!("  端点 {} 连接失败: {}", endpoint, e);
            }
        }
    }

    if project_id.is_empty() {
        project_id = "rising-fact-p41fc".to_string();
        println!("  ⚠ 使用默认 projectId: {}", project_id);
    }

    // Step 4c: Try onboardUser in case needed
    println!("  尝试 onboardUser...");
    for endpoint in &[ENDPOINT_PROD, ENDPOINT_DAILY] {
        let url = format!("{}/v1internal:onboardUser", endpoint);
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "google-api-nodejs-client/9.15.1")
            .json(&serde_json::json!({
                "tierId": "FREE",
                "metadata": {
                    "ideType": "ANTIGRAVITY",
                    "platform": "PLATFORM_UNSPECIFIED",
                    "pluginType": "GEMINI"
                }
            }))
            .send()
            .await;
        match resp {
            Ok(r) => {
                let status = r.status();
                let body_text = r.text().await.unwrap_or_default();
                println!("  onboardUser {}: HTTP {} - {}", endpoint, status, &body_text[..body_text.len().min(200)]);
            }
            Err(e) => println!("  onboardUser {} 失败: {}", endpoint, e),
        }
    }

    // Step 5: Call Gemini API
    println!("\n[5/5] 测试 Gemini API 调用...");

    let request_id = format!("agent-{}", uuid::Uuid::new_v4());

    let api_body = serde_json::json!({
        "project": project_id,
        "model": "gemini-3-flash",
        "request": {
            "contents": [
                {
                    "role": "user",
                    "parts": [{"text": "回复\"验证成功\"这四个字，不要说其他内容。"}]
                }
            ],
            "generationConfig": {
                "maxOutputTokens": 100,
                "temperature": 0.3
            },
            "systemInstruction": {
                "role": "user",
                "parts": [{"text": "You are a helpful assistant. Reply in Chinese."}]
            }
        },
        "requestType": "agent",
        "userAgent": "antigravity",
        "requestId": request_id
    });

    // Try daily endpoint first, then prod
    let endpoints_to_try = [ENDPOINT_PROD, ENDPOINT_DAILY];
    let mut api_success = false;

    for endpoint in &endpoints_to_try {
        let url = format!("{}/v1internal:streamGenerateContent?alt=sse", endpoint);
        println!("  尝试: {}", url);

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "antigravity/1.18.3 darwin/arm64")
            .header("Accept", "text/event-stream")
            .header("X-Goog-Api-Client", "google-cloud-sdk vscode_cloudshelleditor/0.1")
            .header("Client-Metadata", r#"{"ideType":"ANTIGRAVITY","platform":"PLATFORM_UNSPECIFIED","pluginType":"GEMINI"}"#)
            .json(&api_body)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if status.is_success() {
            // Parse SSE response
            let mut full_reply = String::new();
            for line in body.lines() {
                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim();
                    if data.is_empty() { continue; }
                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(candidates) = event.get("response")
                            .or(Some(&event))  // sometimes response is at top level
                            .and_then(|r| r.get("candidates"))
                            .and_then(|c| c.as_array())
                        {
                            for candidate in candidates {
                                if let Some(parts) = candidate
                                    .get("content")
                                    .and_then(|c| c.get("parts"))
                                    .and_then(|p| p.as_array())
                                {
                                    for part in parts {
                                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                            full_reply.push_str(text);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            println!("\n  ========================================");
            println!("  🎉 验证通过！Gemini API 调用成功！");
            println!("  AI 回复: {}", if full_reply.is_empty() { "(空)" } else { &full_reply });
            println!("  HTTP 状态: {}", status);
            println!("  端点: {}", endpoint);
            println!("  ========================================");
            api_success = true;
            break;
        } else {
            println!("  ❌ 失败 (HTTP {})", status);
            let preview: String = body.chars().take(300).collect();
            println!("  {}", preview);
        }
    }

    // Summary
    println!("\n=== 验证结果汇总 ===");
    println!("  OAuth 登录:      ✓");
    println!("  Token 交换:      ✓");
    println!("  refresh_token:   {}", if refresh_token.is_some() { "✓" } else { "✗" });
    println!("  projectId:       {}", if !project_id.is_empty() { "✓" } else { "✗" });
    println!("  Gemini API 调用: {}", if api_success { "✓" } else { "✗" });

    if refresh_token.is_some() {
        println!("\n  [额外] 测试 refresh token...");
        let refresh_resp = client
            .post(TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!(
                "grant_type=refresh_token&refresh_token={}&client_id={}&client_secret={}",
                url_encode(refresh_token.unwrap()),
                url_encode(CLIENT_ID),
                url_encode(CLIENT_SECRET),
            ))
            .send()
            .await?;
        println!("  Token 刷新: {} (HTTP {})",
            if refresh_resp.status().is_success() { "✓" } else { "✗" },
            refresh_resp.status()
        );
    }

    Ok(())
}

async fn wait_for_callback(
    listener: &TcpListener,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let (mut stream, _) = tokio::time::timeout(
        std::time::Duration::from_secs(180),
        listener.accept(),
    )
    .await
    .map_err(|_| "等待授权超时（180秒）")?
    .map_err(|e| format!("接受连接失败: {}", e))?;

    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut request_line = String::new();
    buf_reader.read_line(&mut request_line).await?;

    let path = request_line.split_whitespace().nth(1)
        .ok_or("Invalid HTTP request")?.to_string();

    // Drain headers
    loop {
        let mut line = String::new();
        buf_reader.read_line(&mut line).await?;
        if line.trim().is_empty() { break; }
    }

    // Parse query params
    let query = path.split('?').nth(1).ok_or("No query params")?;
    let mut code = String::new();
    let mut state = String::new();
    for param in query.split('&') {
        let mut kv = param.splitn(2, '=');
        match (kv.next(), kv.next()) {
            (Some("code"), Some(v)) => code = v.to_string(),
            (Some("state"), Some(v)) => state = v.to_string(),
            _ => {}
        }
    }

    // Send success response
    let html = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
        <html><body style='font-family:system-ui;text-align:center;padding:60px'>\
        <h2 style='color:#22c55e'>&#10003; Google 授权成功</h2>\
        <p>已收到授权，请返回小云。</p>\
        <p style='color:#888;font-size:14px'>你可以关闭此页面。</p>\
        </body></html>";

    let _ = writer.write_all(html.as_bytes()).await;
    let _ = writer.shutdown().await;

    if code.is_empty() {
        return Err("未收到授权码".into());
    }

    Ok((code, state))
}
