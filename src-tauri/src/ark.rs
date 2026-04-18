//! 火山引擎方舟 OpenAPI 兼容 Chat Completions（豆包等）。
//! 密钥仅来自环境变量 `ARK_API_KEY`，勿写入源码或提交 .env。

use serde::Deserialize;
use serde_json::json;

const DEFAULT_BASE: &str = "https://ark.cn-beijing.volces.com/api/v3";
const DEFAULT_MODEL: &str = "doubao-seed-2-0-lite-260215";

#[derive(Debug, Deserialize)]
struct ArkResponse {
    choices: Option<Vec<ArkChoice>>,
    error: Option<ArkErrorBody>,
}

#[derive(Debug, Deserialize)]
struct ArkChoice {
    message: Option<ArkMessage>,
}

#[derive(Debug, Deserialize)]
struct ArkMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArkErrorBody {
    message: Option<String>,
    #[allow(dead_code)]
    code: Option<String>,
}

/// 调用方舟 chat completions，返回 assistant 文本。
pub async fn chat_completion(system: &str, user: &str) -> Result<String, String> {
    let key = std::env::var("ARK_API_KEY")
        .map_err(|_| "未设置环境变量 ARK_API_KEY。请在 SchedulePCagent/.env 中配置，或写入系统用户环境变量。".to_string())?;

    let base = std::env::var("ARK_API_BASE").unwrap_or_else(|_| DEFAULT_BASE.to_string());
    let model = std::env::var("ARK_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        if let Ok(v) = serde_json::from_str::<ArkResponse>(&text) {
            if let Some(err) = v.error {
                if let Some(m) = err.message {
                    return Err(format!("API 错误 ({}): {}", status, m));
                }
            }
        }
        return Err(format!("API 错误 ({}): {}", status, text));
    }

    let v: ArkResponse = serde_json::from_str(&text).map_err(|e| format!("解析响应 JSON: {e}, body: {text}"))?;

    if let Some(err) = v.error {
        if let Some(m) = err.message {
            return Err(m);
        }
    }

    let content = v
        .choices
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("响应无 choices[0].message.content: {text}"))?;

    Ok(content)
}

/// 手边助手：支持纯文字或「文字 + 单张图片」（OpenAI 兼容多模态 content 数组）。
/// 若接口或模型不支持图片，请在外层捕获错误后改调 `chat_completion`。
pub async fn chat_completion_with_optional_image(
    system: &str,
    user_text: &str,
    image_mime: &str,
    image_base64: &str,
) -> Result<String, String> {
    let key = std::env::var("ARK_API_KEY")
        .map_err(|_| "未设置环境变量 ARK_API_KEY。请在 SchedulePCagent/.env 中配置，或写入系统用户环境变量。".to_string())?;

    let base = std::env::var("ARK_API_BASE").unwrap_or_else(|_| DEFAULT_BASE.to_string());
    let model = std::env::var("ARK_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let data_url = format!(
        "data:{};base64,{}",
        image_mime.trim(),
        image_base64.trim()
    );

    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            {
                "role": "user",
                "content": [
                    { "type": "image_url", "image_url": { "url": data_url } },
                    { "type": "text", "text": user_text }
                ]
            }
        ]
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        if let Ok(v) = serde_json::from_str::<ArkResponse>(&text) {
            if let Some(err) = v.error {
                if let Some(m) = err.message {
                    return Err(format!("API 错误 ({}): {}", status, m));
                }
            }
        }
        return Err(format!("API 错误 ({}): {}", status, text));
    }

    let v: ArkResponse = serde_json::from_str(&text).map_err(|e| format!("解析响应 JSON: {e}, body: {text}"))?;

    if let Some(err) = v.error {
        if let Some(m) = err.message {
            return Err(m);
        }
    }

    let content = v
        .choices
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("响应无 choices[0].message.content: {text}"))?;

    Ok(content)
}
