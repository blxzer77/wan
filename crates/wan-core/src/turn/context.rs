//! Context assembly seam. The system prompt content is owned by M1-17 and is not defined here.

use std::future::Future;
use std::sync::Arc;

/// `profile.md` as captured when the session started; it is injected unchanged for the
/// whole session so later edits cannot alter an in-flight conversation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FrozenProfile(Arc<str>);

impl FrozenProfile {
    pub fn new(text: impl Into<Arc<str>>) -> Self {
        Self(text.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Prompt material grouped by how often it changes.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ContextLayers {
    pub stable: Vec<String>,
    pub context: Vec<String>,
    pub volatile: Vec<String>,
}

pub trait ContextAssembler: Send + Sync {
    fn assemble(
        &self,
        turn_id: &str,
        profile: &FrozenProfile,
    ) -> impl Future<Output = ContextLayers> + Send;
}
