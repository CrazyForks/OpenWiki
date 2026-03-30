use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// --- Data Types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub index: usize,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceGroup {
    pub label: String,
    pub items: Vec<EvidenceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringThread {
    pub title: String,
    pub summary: String,
    pub evidence: Vec<EvidenceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnexpectedConnection {
    pub title: String,
    pub insight: String,
    pub group_a: EvidenceGroup,
    pub group_b: EvidenceGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewObsession {
    pub title: String,
    pub first_seen: String,
    pub intensity: String,
    pub evidence: Vec<EvidenceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionAnalysis {
    pub recurring_threads: Vec<RecurringThread>,
    pub unexpected_connections: Vec<UnexpectedConnection>,
    pub new_obsessions: Vec<NewObsession>,
}

// --- Prompt Builder ---

/// Build system prompt and user message from content items.
/// Each item is (id, raw_text, source_url, captured_at).
/// Returns (system_prompt, user_message).
pub fn build_prompt(
    items: &[(String, Option<String>, Option<String>, String)],
) -> (String, String) {
    let count = items.len();
    let max_chars: usize = if count <= 20 {
        1000
    } else if count <= 50 {
        600
    } else if count <= 100 {
        400
    } else {
        300
    };

    let system_prompt = r#"你是一个注意力分析助手。你的任务是分析用户最近收集的内容，找出三种模式：

1. **recurring_threads**（反复出现的主题）：至少有3条内容涉及同一个话题。找出用户持续关注的主题。
2. **unexpected_connections**（意想不到的联系）：找出两组看似无关但实际上有隐藏联系的内容。
3. **new_obsessions**（新的兴趣点）：最近3-5天内出现的新兴趣，之前没有出现过的主题。

重要规则：
- 使用内容的**序号**（从0开始的index）来引用内容，不要使用内容ID。
- 每个evidence item必须包含index（序号）和snippet（内容摘要）。
- unexpected_connections的group_a和group_b各自包含label和items。
- 如果某个类别没有发现模式，返回空数组。

请严格按以下JSON格式返回，不要包含其他内容：
{
  "recurring_threads": [
    {
      "title": "主题标题",
      "summary": "主题总结",
      "evidence": [
        {"index": 0, "snippet": "内容摘要"}
      ]
    }
  ],
  "unexpected_connections": [
    {
      "title": "联系标题",
      "insight": "联系洞察",
      "group_a": {
        "label": "分组A标签",
        "items": [{"index": 0, "snippet": "内容摘要"}]
      },
      "group_b": {
        "label": "分组B标签",
        "items": [{"index": 1, "snippet": "内容摘要"}]
      }
    }
  ],
  "new_obsessions": [
    {
      "title": "新兴趣标题",
      "first_seen": "2024-01-01",
      "intensity": "高/中/低",
      "evidence": [
        {"index": 0, "snippet": "内容摘要"}
      ]
    }
  ]
}"#
        .to_string();

    let mut content_lines = Vec::with_capacity(count);
    for (i, (id, raw_text, source_url, captured_at)) in items.iter().enumerate() {
        let text = raw_text.as_deref().unwrap_or("[无文本]");
        let truncated = truncate_str(text, max_chars);
        let url_part = source_url
            .as_deref()
            .map(|u| format!(" | 来源: {}", u))
            .unwrap_or_default();
        content_lines.push(format!(
            "[{}] (id={}, 时间={}{}) {}",
            i, id, captured_at, url_part, truncated
        ));
    }

    let user_message = format!(
        "以下是用户最近收集的{}条内容，请分析其中的注意力模式：\n\n{}",
        count,
        content_lines.join("\n\n")
    );

    (system_prompt, user_message)
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars].iter().collect();
        format!("{}...", truncated)
    }
}

// --- JSON Validator ---

/// Parse and validate an AttentionAnalysis JSON string.
/// Drops evidence items with out-of-bounds index.
/// Drops threads/connections/obsessions with no remaining evidence.
pub fn validate_analysis(json_str: &str, item_count: usize) -> Result<AttentionAnalysis, String> {
    // Try to extract JSON from markdown code blocks if present
    let cleaned = extract_json(json_str);

    let mut analysis: AttentionAnalysis = serde_json::from_str(&cleaned)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;

    // Filter recurring_threads
    analysis.recurring_threads.retain_mut(|thread| {
        thread.evidence.retain(|e| e.index < item_count);
        !thread.evidence.is_empty()
    });

    // Filter unexpected_connections
    analysis.unexpected_connections.retain_mut(|conn| {
        conn.group_a.items.retain(|e| e.index < item_count);
        conn.group_b.items.retain(|e| e.index < item_count);
        !conn.group_a.items.is_empty() || !conn.group_b.items.is_empty()
    });

    // Filter new_obsessions
    analysis.new_obsessions.retain_mut(|obs| {
        obs.evidence.retain(|e| e.index < item_count);
        !obs.evidence.is_empty()
    });

    Ok(analysis)
}

fn extract_json(s: &str) -> String {
    let trimmed = s.trim();
    // Check for ```json ... ``` blocks
    if let Some(start) = trimmed.find("```json") {
        let after_marker = &trimmed[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim().to_string();
        }
    }
    // Check for ``` ... ``` blocks
    if let Some(start) = trimmed.find("```") {
        let after_marker = &trimmed[start + 3..];
        if let Some(end) = after_marker.find("```") {
            return after_marker[..end].trim().to_string();
        }
    }
    trimmed.to_string()
}

// --- API Caller ---

/// Supported provider for direct API calls
#[derive(Debug, Clone)]
pub enum AnalysisProvider {
    Anthropic,
    OpenAi,
    OpenRouter,
}

impl AnalysisProvider {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" => AnalysisProvider::OpenAi,
            "openrouter" => AnalysisProvider::OpenRouter,
            _ => AnalysisProvider::Anthropic,
        }
    }
}

// --- Anthropic types (local to this module) ---

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ApiMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    text: String,
}

// --- OpenAI types (local to this module) ---

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    max_tokens: u32,
    temperature: f32,
    response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: ApiMessage,
}

/// Call the AI API directly to perform attention analysis.
/// Returns the raw response text.
pub async fn call_analysis_api(
    provider: &AnalysisProvider,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_message: &str,
) -> Result<String, String> {
    let http_client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client 创建失败: {}", e))?;

    match provider {
        AnalysisProvider::Anthropic => {
            let body = AnthropicRequest {
                model: model.to_string(),
                max_tokens: 4096,
                system: system_prompt.to_string(),
                messages: vec![ApiMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                }],
            };

            let resp = http_client
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Anthropic API 请求失败: {}", e))?;

            let status = resp.status();
            let text = resp
                .text()
                .await
                .map_err(|e| format!("读取 Anthropic 响应失败: {}", e))?;

            if !status.is_success() {
                return Err(format!("Anthropic API 错误 ({}): {}", status, text));
            }

            let parsed: AnthropicResponse = serde_json::from_str(&text)
                .map_err(|e| format!("解析 Anthropic 响应失败: {}", e))?;

            Ok(parsed
                .content
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_default())
        }
        AnalysisProvider::OpenAi | AnalysisProvider::OpenRouter => {
            let url = match provider {
                AnalysisProvider::OpenRouter => {
                    "https://openrouter.ai/api/v1/chat/completions"
                }
                _ => "https://api.openai.com/v1/chat/completions",
            };

            let body = OpenAiRequest {
                model: model.to_string(),
                messages: vec![
                    ApiMessage {
                        role: "system".to_string(),
                        content: system_prompt.to_string(),
                    },
                    ApiMessage {
                        role: "user".to_string(),
                        content: user_message.to_string(),
                    },
                ],
                max_tokens: 4096,
                temperature: 0.3,
                response_format: ResponseFormat {
                    format_type: "json_object".to_string(),
                },
            };

            let mut req = http_client
                .post(url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json");

            if matches!(provider, AnalysisProvider::OpenRouter) {
                req = req
                    .header("HTTP-Referer", "https://xiaoyun.app")
                    .header("X-Title", "Xiaoyun");
            }

            let resp = req
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("API 请求失败: {}", e))?;

            let status = resp.status();
            let text = resp
                .text()
                .await
                .map_err(|e| format!("读取 API 响应失败: {}", e))?;

            if !status.is_success() {
                return Err(format!("API 错误 ({}): {}", status, text));
            }

            let parsed: OpenAiResponse = serde_json::from_str(&text)
                .map_err(|e| format!("解析 API 响应失败: {}", e))?;

            Ok(parsed
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default())
        }
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_json(max_index: usize) -> String {
        format!(
            r#"{{
  "recurring_threads": [
    {{
      "title": "Rust学习",
      "summary": "用户持续关注Rust语言",
      "evidence": [
        {{"index": 0, "snippet": "Rust所有权"}},
        {{"index": 1, "snippet": "Rust生命周期"}},
        {{"index": 2, "snippet": "Rust异步编程"}}
      ]
    }}
  ],
  "unexpected_connections": [
    {{
      "title": "编程与音乐",
      "insight": "两者都涉及模式识别",
      "group_a": {{
        "label": "编程",
        "items": [{{"index": 0, "snippet": "代码模式"}}]
      }},
      "group_b": {{
        "label": "音乐",
        "items": [{{"index": {}, "snippet": "音乐模式"}}]
      }}
    }}
  ],
  "new_obsessions": [
    {{
      "title": "AI绘画",
      "first_seen": "2024-03-25",
      "intensity": "高",
      "evidence": [
        {{"index": 1, "snippet": "Stable Diffusion"}}
      ]
    }}
  ]
}}"#,
            max_index
        )
    }

    #[test]
    fn test_validate_analysis_valid() {
        let json = make_valid_json(3);
        let result = validate_analysis(&json, 5);
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.recurring_threads.len(), 1);
        assert_eq!(analysis.recurring_threads[0].evidence.len(), 3);
        assert_eq!(analysis.unexpected_connections.len(), 1);
        assert_eq!(analysis.new_obsessions.len(), 1);
    }

    #[test]
    fn test_validate_analysis_out_of_bounds() {
        // item_count = 2, so index 2 and 3 are out of bounds
        let json = make_valid_json(3);
        let result = validate_analysis(&json, 2);
        assert!(result.is_ok());
        let analysis = result.unwrap();

        // recurring_threads: index 0, 1 survive; index 2 dropped. Still has evidence.
        assert_eq!(analysis.recurring_threads.len(), 1);
        assert_eq!(analysis.recurring_threads[0].evidence.len(), 2);

        // unexpected_connections: group_a index 0 survives, group_b index 3 dropped
        assert_eq!(analysis.unexpected_connections.len(), 1);
        assert_eq!(analysis.unexpected_connections[0].group_a.items.len(), 1);
        assert_eq!(analysis.unexpected_connections[0].group_b.items.len(), 0);

        // new_obsessions: index 1 survives
        assert_eq!(analysis.new_obsessions.len(), 1);
        assert_eq!(analysis.new_obsessions[0].evidence.len(), 1);
    }

    #[test]
    fn test_validate_analysis_all_out_of_bounds() {
        // item_count = 0, all indices are out of bounds
        let json = make_valid_json(3);
        let result = validate_analysis(&json, 0);
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.recurring_threads.is_empty());
        assert!(analysis.unexpected_connections.is_empty());
        assert!(analysis.new_obsessions.is_empty());
    }

    #[test]
    fn test_validate_analysis_invalid_json() {
        let result = validate_analysis("not json at all", 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("JSON 解析失败"));
    }

    #[test]
    fn test_validate_analysis_empty_arrays() {
        let json = r#"{
            "recurring_threads": [],
            "unexpected_connections": [],
            "new_obsessions": []
        }"#;
        let result = validate_analysis(json, 5);
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.recurring_threads.is_empty());
        assert!(analysis.unexpected_connections.is_empty());
        assert!(analysis.new_obsessions.is_empty());
    }

    #[test]
    fn test_validate_analysis_markdown_wrapped() {
        let json = format!("```json\n{}\n```", make_valid_json(2));
        let result = validate_analysis(&json, 5);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_prompt_truncation_small() {
        // <=20 items -> 1000 char limit
        let items: Vec<(String, Option<String>, Option<String>, String)> = (0..5)
            .map(|i| {
                (
                    format!("id-{}", i),
                    Some("a".repeat(1500)),
                    Some(format!("https://example.com/{}", i)),
                    "2024-03-25".to_string(),
                )
            })
            .collect();

        let (system, user) = build_prompt(&items);
        assert!(!system.is_empty());
        assert!(user.contains("[0]"));
        assert!(user.contains("[4]"));
        // Each item text should be truncated to 1000 chars + "..."
        // The "a".repeat(1500) should become "a".repeat(1000) + "..."
        assert!(user.contains(&"a".repeat(1000)));
        assert!(!user.contains(&"a".repeat(1001)));
    }

    #[test]
    fn test_build_prompt_truncation_medium() {
        // 21-50 items -> 600 char limit
        let items: Vec<(String, Option<String>, Option<String>, String)> = (0..25)
            .map(|i| {
                (
                    format!("id-{}", i),
                    Some("b".repeat(1000)),
                    None,
                    "2024-03-25".to_string(),
                )
            })
            .collect();

        let (_system, user) = build_prompt(&items);
        assert!(user.contains(&"b".repeat(600)));
        assert!(!user.contains(&"b".repeat(601)));
    }

    #[test]
    fn test_build_prompt_truncation_large() {
        // 51-100 items -> 400 char limit
        let items: Vec<(String, Option<String>, Option<String>, String)> = (0..60)
            .map(|i| {
                (
                    format!("id-{}", i),
                    Some("c".repeat(800)),
                    None,
                    "2024-03-25".to_string(),
                )
            })
            .collect();

        let (_system, user) = build_prompt(&items);
        assert!(user.contains(&"c".repeat(400)));
        assert!(!user.contains(&"c".repeat(401)));
    }

    #[test]
    fn test_build_prompt_truncation_xlarge() {
        // >100 items -> 300 char limit
        let items: Vec<(String, Option<String>, Option<String>, String)> = (0..110)
            .map(|i| {
                (
                    format!("id-{}", i),
                    Some("d".repeat(500)),
                    None,
                    "2024-03-25".to_string(),
                )
            })
            .collect();

        let (_system, user) = build_prompt(&items);
        assert!(user.contains(&"d".repeat(300)));
        assert!(!user.contains(&"d".repeat(301)));
    }

    #[test]
    fn test_build_prompt_no_text() {
        let items = vec![(
            "id-0".to_string(),
            None,
            None,
            "2024-03-25".to_string(),
        )];
        let (_system, user) = build_prompt(&items);
        assert!(user.contains("[无文本]"));
    }

    #[test]
    fn test_build_prompt_short_text_no_truncation() {
        let items = vec![(
            "id-0".to_string(),
            Some("短文本".to_string()),
            None,
            "2024-03-25".to_string(),
        )];
        let (_system, user) = build_prompt(&items);
        assert!(user.contains("短文本"));
        assert!(!user.contains("..."));
    }
}
