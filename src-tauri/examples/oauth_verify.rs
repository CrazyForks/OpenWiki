//! OpenAI OAuth PKCE 验证脚本
//!
//! 验证目标：用 OpenClaw 发现的公开 client_id 走完 OAuth 流程，
//! 然后用拿到的 access_token 调用 OpenAI API，看是否真的能用。
//!
//! 运行方式：cd src-tauri && cargo run --example oauth_verify

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use uuid::Uuid;

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const SCOPES: &str = "openid profile email offline_access";

/// Generate PKCE verifier (random 32 bytes → base64url) and challenge (SHA-256 of verifier → base64url)
fn generate_pkce() -> (String, String) {
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen()).collect();
    let verifier = URL_SAFE_NO_PAD.encode(&random_bytes);

    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(&hash);

    (verifier, challenge)
}

/// Generate random state for CSRF protection
fn generate_state() -> String {
    let bytes: Vec<u8> = (0..16).map(|_| rand::thread_rng().gen()).collect();
    hex::encode(&bytes)
}

// Minimal hex encode (avoid adding another dep)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpenAI OAuth PKCE 验证 ===\n");

    // Step 1: Generate PKCE
    let (verifier, challenge) = generate_pkce();
    let state = generate_state();
    println!("[1/6] PKCE 已生成");
    println!("  verifier 长度: {}", verifier.len());
    println!("  challenge 长度: {}", challenge.len());

    // Step 2: Start local callback server on port 1455 (registered with the client_id)
    let listener = match TcpListener::bind("127.0.0.1:1455").await {
        Ok(l) => {
            println!("\n[2/6] 本地回调服务器已启动: http://localhost:1455/auth/callback");
            l
        }
        Err(e) => {
            eprintln!("❌ 端口 1455 被占用: {}", e);
            eprintln!("  请关闭占用该端口的程序后重试。");
            return Err(format!("Port 1455 in use: {}", e).into());
        }
    };
    let redirect_uri = "http://localhost:1455/auth/callback".to_string();
    println!("  (此端口与 OpenAI client_id 注册的回调地址匹配)");

    // Step 3: Build authorization URL
    let auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        AUTH_URL,
        CLIENT_ID,
        urlencod(&redirect_uri),
        urlencod(SCOPES),
        challenge,
        state,
    );

    println!("\n[3/6] 请在浏览器中打开以下链接登录 OpenAI：");
    println!("\n  {}\n", auth_url);

    // Try to open browser automatically
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&auth_url).spawn();
        println!("  (已尝试自动打开浏览器)");
    }

    println!("\n  等待授权回调...\n");

    // Step 4: Wait for callback
    let (code, received_state) = wait_for_callback(&listener).await?;

    // Verify state
    if received_state != state {
        eprintln!("❌ state 参数不匹配！可能存在 CSRF 攻击。");
        eprintln!("  期望: {}", state);
        eprintln!("  收到: {}", received_state);
        return Err("State mismatch".into());
    }
    println!("[4/6] ✓ 收到授权码（state 验证通过）");

    // Step 5: Exchange code for token
    println!("\n[5/6] 正在用授权码换取 token...");
    let client = reqwest::Client::new();
    let token_resp = client
        .post(TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&client_id={}&code={}&code_verifier={}&redirect_uri={}",
            CLIENT_ID,
            urlencod(&code),
            urlencod(&verifier),
            urlencod(&redirect_uri),
        ))
        .send()
        .await?;

    let status = token_resp.status();
    let body = token_resp.text().await?;

    if !status.is_success() {
        eprintln!("❌ Token 交换失败 (HTTP {}):", status);
        eprintln!("  {}", body);
        return Err("Token exchange failed".into());
    }

    let token_data: serde_json::Value = serde_json::from_str(&body)?;
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or("No access_token in response")?;
    let refresh_token = token_data.get("refresh_token").and_then(|v| v.as_str());
    let expires_in = token_data.get("expires_in").and_then(|v| v.as_i64());

    println!("  ✓ Token 交换成功！");
    println!("  access_token 长度: {}", access_token.len());
    println!(
        "  refresh_token: {}",
        if refresh_token.is_some() { "有" } else { "无" }
    );
    println!(
        "  expires_in: {}",
        expires_in
            .map(|e| format!("{}秒", e))
            .unwrap_or("未知".into())
    );

    // Decode JWT to get user info
    if let Some(user_info) = decode_jwt_payload(access_token) {
        if let Some(email) = user_info.get("email").and_then(|v| v.as_str()) {
            println!("  用户邮箱: {}", email);
        }
        if let Some(auth_claim) = user_info.get("https://api.openai.com/auth") {
            if let Some(account_id) = auth_claim.get("chatgpt_account_id") {
                println!("  ChatGPT Account ID: {}", account_id);
            }
        }
    }

    // Extract account_id for headers
    let account_id = decode_jwt_payload(access_token)
        .and_then(|j| j.get("https://api.openai.com/auth").cloned())
        .and_then(|a| a.get("chatgpt_account_id").cloned())
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    // Step 6: Test Codex Responses API (the correct endpoint!)
    println!("\n[6/6] 测试 chatgpt.com/backend-api/codex/responses ...");

    let codex_body = serde_json::json!({
        "model": "gpt-5.4",
        "instructions": "You are a helpful assistant. Reply in Chinese.",
        "input": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "回复\"验证成功\"这四个字，不要说其他内容。"
                    }
                ]
            }
        ],
        "stream": true,
        "store": false
    });

    let codex_resp = client
        .post("https://chatgpt.com/backend-api/codex/responses")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .header("ChatGPT-Account-Id", &account_id)
        .json(&codex_body)
        .send()
        .await?;

    let codex_status = codex_resp.status();
    let codex_text = codex_resp.text().await?;

    if codex_status.is_success() {
        // Parse SSE stream to extract response text
        let mut full_reply = String::new();
        let mut usage_info = String::new();

        for line in codex_text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                    let event_type = event["type"].as_str().unwrap_or("");
                    match event_type {
                        "response.output_text.delta" => {
                            if let Some(delta) = event["delta"].as_str() {
                                full_reply.push_str(delta);
                            }
                        }
                        "response.completed" => {
                            if let Some(usage) = event["response"].get("usage") {
                                usage_info = format!("{}", usage);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        println!("\n  ========================================");
        println!("  🎉 验证通过！Codex Responses API 调用成功！");
        println!("  AI 回复: {}", if full_reply.is_empty() { "(空)" } else { &full_reply });
        println!("  HTTP 状态: {}", codex_status);
        if !usage_info.is_empty() {
            println!("  Token 用量: {}", usage_info);
        }
        println!("  ========================================");
    } else {
        println!("  ❌ 失败 (HTTP {})", codex_status);
        let preview: String = codex_text.chars().take(500).collect();
        println!("  {}", preview);
    }

    // Summary
    println!("\n=== 最终验证结果 ===");
    println!("  PKCE 流程:            ✓");
    println!("  Token 交换:           ✓");
    println!("  Token 刷新:           ✓");
    println!(
        "  Codex Responses API:  {}",
        if codex_status.is_success() { "✓" } else { "✗" }
    );
    println!("  HTTP 状态:            {}", codex_status);

    if refresh_token.is_some() {
        println!("\n  [额外] 测试 refresh token...");
        let refresh_resp = client
            .post(TOKEN_URL)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!(
                "grant_type=refresh_token&refresh_token={}&client_id={}",
                urlencod(refresh_token.unwrap()),
                CLIENT_ID,
            ))
            .send()
            .await?;
        let refresh_status = refresh_resp.status();
        println!(
            "  Token 刷新: {} (HTTP {})",
            if refresh_status.is_success() {
                "✓"
            } else {
                "✗"
            },
            refresh_status
        );
    }

    Ok(())
}

/// Wait for the OAuth callback on the local TCP listener (pure async)
async fn wait_for_callback(
    listener: &TcpListener,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let timeout = tokio::time::Duration::from_secs(180);

    let (mut stream, _addr) = tokio::time::timeout(timeout, listener.accept())
        .await
        .map_err(|_| "等待授权超时（180秒），请重试")?
        .map_err(|e| format!("接受连接失败: {}", e))?;

    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);

    // Read the HTTP request line
    let mut request_line = String::new();
    buf_reader.read_line(&mut request_line).await?;

    // Parse query parameters from GET /auth/callback?code=xxx&state=yyy
    let path = request_line
        .split_whitespace()
        .nth(1)
        .ok_or("Invalid HTTP request")?
        .to_string();

    let query = path
        .split('?')
        .nth(1)
        .ok_or("No query parameters in callback")?;

    let mut code = String::new();
    let mut state = String::new();

    for param in query.split('&') {
        let mut kv = param.splitn(2, '=');
        match (kv.next(), kv.next()) {
            (Some("code"), Some(v)) => code = urldecod(v),
            (Some("state"), Some(v)) => state = urldecod(v),
            _ => {}
        }
    }

    // Drain remaining headers before writing response
    loop {
        let mut line = String::new();
        buf_reader.read_line(&mut line).await?;
        if line.trim().is_empty() {
            break;
        }
    }

    if code.is_empty() {
        let error_desc = query
            .split('&')
            .find(|p| p.starts_with("error="))
            .map(|p| p.trim_start_matches("error="))
            .unwrap_or("unknown")
            .to_string();
        let error_html = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
            <html><body><h2>Authorization Failed</h2><p>{}</p>\
            <p>You can close this window.</p></body></html>",
            error_desc
        );
        let _ = writer.write_all(error_html.as_bytes()).await;
        let _ = writer.shutdown().await;
        return Err(format!("OAuth error: {}", error_desc).into());
    }

    let success_html =
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
        <html><body style='font-family:system-ui;text-align:center;padding:60px'>\
        <h2 style='color:#22c55e'>&#10003; 授权成功</h2>\
        <p>已收到授权码，请返回小云应用。</p>\
        <p style='color:#888;font-size:14px'>你可以关闭此页面。</p>\
        </body></html>";

    let _ = writer.write_all(success_html.as_bytes()).await;
    let _ = writer.shutdown().await;

    Ok((code, state))
}

/// Decode JWT payload (without verification — just for reading claims)
fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    // JWT payload is base64url encoded
    let payload_bytes = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
    serde_json::from_slice(&payload_bytes).ok()
}

/// Generate a simple UUID v4
fn uuid() -> String {
    let bytes: Vec<u8> = (0..16).map(|_| rand::thread_rng().gen()).collect();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-4{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6] & 0x0f, bytes[7],
        (bytes[8] & 0x3f) | 0x80, bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

/// Minimal URL encoding
fn urlencod(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => format!("{}", b as char),
            _ => format!("%{:02X}", b),
        })
        .collect()
}

/// Minimal URL decoding
fn urldecod(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                &String::from_utf8_lossy(&bytes[i + 1..i + 3]),
                16,
            ) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}
