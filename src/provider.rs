use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Context {
    pub prompt: String,
    pub pwd: String,
    pub session_id: String,
    pub history: Vec<String>,
    pub last_command: Option<String>,
}

#[async_trait::async_trait]
pub trait CommandGenerator: Send + Sync {
    async fn generate(&self, context: Context) -> Result<String>;
}

use crate::config::Config;

pub fn build_provider(config: &Config) -> Result<Box<dyn CommandGenerator>> {
    match config.provider.type_.as_str() {
        "openai" | "openai-compatible" => Ok(Box::new(OpenAiCompatibleProvider::new(config)?)),
        "noop" => Ok(Box::new(NoopProvider)),
        _ => Err(anyhow!("unknown provider: {}", config.provider.type_)),
    }
}

pub struct NoopProvider;

#[async_trait::async_trait]
impl CommandGenerator for NoopProvider {
    async fn generate(&self, context: Context) -> Result<String> {
        let prompt = context.prompt.trim();
        if prompt.is_empty() {
            return Ok(":".to_string());
        }
        Ok(format!("echo \"Chitin: {prompt}\""))
    }
}

pub struct OpenAiCompatibleProvider {
    base_url: String,
    api_key: String,
    model: String,
    client: Client,
}

impl OpenAiCompatibleProvider {
    pub fn new(config: &Config) -> Result<Self> {
        let base_url = config
            .provider
            .openai
            .api_base
            .clone()
            .unwrap_or_else(|| "https://api.openai.com".to_string());

        let api_key = config
            .provider
            .openai
            .api_key
            .clone()
            .ok_or_else(|| anyhow!("API key is required for openai provider"))?;

        let model = config
            .provider
            .openai
            .model
            .clone()
            .unwrap_or_else(|| "gpt-4.1-mini".to_string());

        let client = Client::new();
        Ok(Self {
            base_url,
            api_key,
            model,
            client,
        })
    }

    fn build_prompt(&self, context: &Context) -> Vec<Message> {
        let mut details = vec![format!("pwd: {}", context.pwd)];
        if let Some(last) = &context.last_command {
            details.push(format!("last_command: {last}"));
        }
        if !context.history.is_empty() {
            details.push(format!("recent_prompts: {}", context.history.join(" | ")));
        }

        let system = Message {
            role: "system".to_string(),
            content: "You are a shell command generator. Return exactly one executable command, no commentary, no markdown.".to_string(),
        };
        let user = Message {
            role: "user".to_string(),
            content: format!("Task: {}\nContext: {}", context.prompt, details.join("; ")),
        };
        vec![system, user]
    }
}

#[async_trait::async_trait]
impl CommandGenerator for OpenAiCompatibleProvider {
    async fn generate(&self, context: Context) -> Result<String> {
        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );
        let request = ChatRequest {
            model: self.model.clone(),
            messages: self.build_prompt(&context),
            temperature: Some(0.2),
        };

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let payload: ChatResponse = response.json().await?;
        let content = payload
            .choices
            .get(0)
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow!("model response missing content"))?;

        let command = content.lines().next().unwrap_or("").trim();
        if command.is_empty() {
            return Err(anyhow!("model returned empty command"));
        }
        Ok(command.to_string())
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}
