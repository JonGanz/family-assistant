//! Types shared by the decision pipeline and its evaluators (plan/M3.md, Pipeline engine).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::mail::SubjectType;

/// One proposed value for a decision point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub value: serde_json::Value,
    pub confidence: f32,
    /// Signal key or rule id.
    pub source: String,
}

/// What a decision is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Subject {
    pub kind: SubjectType,
    pub id: Uuid,
}

impl Subject {
    pub fn message(id: crate::ids::MessageId) -> Self {
        Self {
            kind: SubjectType::Message,
            id: id.0,
        }
    }

    pub fn event(id: crate::ids::EventId) -> Self {
        Self {
            kind: SubjectType::Event,
            id: id.0,
        }
    }
}
