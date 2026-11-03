//! Message and streaming-event types, modelled on pi-ai.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolCall(ToolCall),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Message {
    User {
        text: String,
    },
    Assistant {
        content: Vec<ContentBlock>,
    },
    ToolResult {
        call_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequest {
    pub model: String,
    pub system: Option<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSpec>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Stop,
    Length,
    ToolUse,
    ContentFilter,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// Prices in USD per million tokens, supplied by the user's model configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Pricing {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Cost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

impl Usage {
    pub fn cost(&self, pricing: &Pricing) -> Cost {
        let per_million = |tokens: u64, price: f64| tokens as f64 * price / 1_000_000.0;
        let input = per_million(self.input, pricing.input);
        let output = per_million(self.output, pricing.output);
        let cache_read = per_million(self.cache_read, pricing.cache_read);
        let cache_write = per_million(self.cache_write, pricing.cache_write);
        Cost {
            input,
            output,
            cache_read,
            cache_write,
            total: input + output + cache_read + cache_write,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: Usage,
    pub cost: Cost,
    pub response_id: Option<String>,
}

/// Incremental output. `index` is the position of the block in the final message content.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    TextDelta {
        index: usize,
        delta: String,
    },
    ToolCallStart {
        index: usize,
        id: String,
        name: String,
    },
    ToolCallDelta {
        index: usize,
        delta: String,
    },
    ToolCallEnd {
        index: usize,
        call: ToolCall,
    },
    Done(AssistantMessage),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_is_priced_per_million_tokens() {
        let usage = Usage {
            input: 2_000_000,
            output: 500_000,
            cache_read: 1_000_000,
            cache_write: 0,
        };
        let pricing = Pricing {
            input: 3.0,
            output: 15.0,
            cache_read: 0.3,
            cache_write: 3.75,
        };

        let cost = usage.cost(&pricing);

        assert_eq!(cost.input, 6.0);
        assert_eq!(cost.output, 7.5);
        assert!((cost.cache_read - 0.3).abs() < 1e-9);
        assert_eq!(cost.cache_write, 0.0);
        assert!((cost.total - 13.8).abs() < 1e-9);
    }
}
