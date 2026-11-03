//! Accumulates the content blocks of one streamed response. Blocks are numbered in order of
//! first appearance, and the same numbers index the final `AssistantMessage::content`, so a
//! stream event's `index` always points at the block it describes.

use serde_json::Value;

use super::types::{ContentBlock, StreamEvent, ToolCall};
use super::{ModelError, Out, parse_arguments};

struct PendingCall {
    id: String,
    name: String,
    arguments: String,
    parsed: Option<Value>,
}

enum Block {
    Text(String),
    Call(PendingCall),
}

#[derive(Default)]
pub(crate) struct Blocks {
    items: Vec<Block>,
}

impl Blocks {
    /// Starts a text block. Callers create it on the first non-empty text so that the final
    /// content never holds an empty text block.
    pub fn push_text(&mut self) -> usize {
        self.items.push(Block::Text(String::new()));
        self.items.len() - 1
    }

    pub fn push_call(&mut self, id: String, name: String) -> usize {
        self.items.push(Block::Call(PendingCall {
            id,
            name,
            arguments: String::new(),
            parsed: None,
        }));
        self.items.len() - 1
    }

    pub fn text_mut(&mut self, index: usize) -> Option<&mut String> {
        match self.items.get_mut(index) {
            Some(Block::Text(text)) => Some(text),
            _ => None,
        }
    }

    pub fn set_call_identity(&mut self, index: usize, id: Option<&str>, name: Option<&str>) {
        if let Some(Block::Call(call)) = self.items.get_mut(index) {
            if let Some(id) = id {
                call.id = id.to_owned();
            }
            if let Some(name) = name {
                call.name = name.to_owned();
            }
        }
    }

    pub fn append_arguments(&mut self, index: usize, delta: &str) -> bool {
        match self.items.get_mut(index) {
            Some(Block::Call(call)) => {
                call.arguments.push_str(delta);
                true
            }
            _ => false,
        }
    }

    /// Replaces the accumulated arguments with the provider's final text.
    pub fn set_arguments(&mut self, index: usize, arguments: &str) {
        if let Some(Block::Call(call)) = self.items.get_mut(index) {
            call.arguments = arguments.to_owned();
        }
    }

    /// Parses the call's arguments and emits `ToolCallEnd` once.
    pub fn end_call(&mut self, index: usize, out: &mut Out) {
        let Some(Block::Call(call)) = self.items.get_mut(index) else {
            return;
        };
        if call.parsed.is_some() {
            return;
        }
        match parse_arguments(&call.arguments) {
            Ok(arguments) => {
                call.parsed = Some(arguments.clone());
                out.push(Ok(StreamEvent::ToolCallEnd {
                    index,
                    call: ToolCall {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        arguments,
                    },
                }));
            }
            Err(error) => out.push(Err(error)),
        }
    }

    pub fn end_all_calls(&mut self, out: &mut Out) {
        for index in 0..self.items.len() {
            self.end_call(index, out);
        }
    }

    /// Returns the final content and whether it contains a tool call.
    pub fn into_content(self) -> Result<(Vec<ContentBlock>, bool), ModelError> {
        let mut has_calls = false;
        let mut content = Vec::with_capacity(self.items.len());
        for block in self.items {
            content.push(match block {
                Block::Text(text) => ContentBlock::Text { text },
                Block::Call(call) => {
                    has_calls = true;
                    let arguments = match call.parsed {
                        Some(arguments) => arguments,
                        None => parse_arguments(&call.arguments)?,
                    };
                    ContentBlock::ToolCall(ToolCall {
                        id: call.id,
                        name: call.name,
                        arguments,
                    })
                }
            });
        }
        Ok((content, has_calls))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn indexes_follow_first_appearance_and_match_final_content() {
        let mut blocks = Blocks::default();
        let call = blocks.push_call("call_1".to_owned(), "read_file".to_owned());
        let text = blocks.push_text();
        blocks.append_arguments(call, "{\"path\":");
        blocks.append_arguments(call, "\"a\"}");
        blocks.text_mut(text).expect("text block").push_str("done");
        let mut out = Vec::new();

        blocks.end_all_calls(&mut out);
        let (content, has_calls) = blocks.into_content().expect("arguments parse");

        assert_eq!((call, text), (0, 1));
        assert!(has_calls);
        assert!(matches!(
            out.as_slice(),
            [Ok(StreamEvent::ToolCallEnd { index: 0, .. })]
        ));
        assert_eq!(
            content,
            vec![
                ContentBlock::ToolCall(ToolCall {
                    id: "call_1".to_owned(),
                    name: "read_file".to_owned(),
                    arguments: json!({"path": "a"}),
                }),
                ContentBlock::Text {
                    text: "done".to_owned()
                },
            ]
        );
    }

    #[test]
    fn ending_a_call_twice_emits_one_event_and_bad_json_is_an_error() {
        let mut blocks = Blocks::default();
        let good = blocks.push_call("a".to_owned(), "f".to_owned());
        let bad = blocks.push_call("b".to_owned(), "g".to_owned());
        blocks.append_arguments(bad, "{not json");
        let mut out = Vec::new();

        blocks.end_call(good, &mut out);
        blocks.end_call(good, &mut out);
        blocks.end_call(bad, &mut out);

        assert_eq!(out.len(), 2);
        assert!(out[0].is_ok());
        assert!(matches!(out[1], Err(ModelError::Protocol(_))));
    }
}
