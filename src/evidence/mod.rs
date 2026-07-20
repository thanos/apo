//! Normalized evidence model for hygiene findings.

use serde::{Deserialize, Serialize};

/// Hygiene category groupings (v0.1 scoring rubric).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Documentation & onboarding (foundation).
    Documentation,
    /// Development hygiene (daily developer experience).
    LocalDevelopment,
    /// Quality assurance (code health).
    Testing,
    /// Security & supply chain (risk reduction).
    Security,
    /// Automation & delivery (velocity + reliability).
    Delivery,
    /// Project management & collaboration (process maturity).
    Collaboration,
}

impl Category {
    /// Human-readable display name (scoring rubric).
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Documentation => "Documentation & Onboarding",
            Self::LocalDevelopment => "Development Hygiene",
            Self::Testing => "Quality Assurance",
            Self::Security => "Security & Supply Chain",
            Self::Delivery => "Automation & Delivery",
            Self::Collaboration => "Project Management & Collaboration",
        }
    }

    /// Stable category id used in scores and reports.
    pub fn id(self) -> &'static str {
        match self {
            Self::Documentation => "documentation",
            Self::LocalDevelopment => "local_development",
            Self::Testing => "testing",
            Self::Security => "security",
            Self::Delivery => "delivery",
            Self::Collaboration => "collaboration",
        }
    }

    /// Rubric weight for the overall score (sums to 1.0 across all categories).
    pub fn weight(self) -> f64 {
        match self {
            Self::Documentation => 0.20,
            Self::LocalDevelopment => 0.15,
            Self::Testing => 0.20,
            Self::Security => 0.15,
            Self::Delivery => 0.15,
            Self::Collaboration => 0.15,
        }
    }

    /// Short rubric rationale for this category.
    pub fn rubric_note(self) -> &'static str {
        match self {
            Self::Documentation => "Foundation",
            Self::LocalDevelopment => "Daily developer experience",
            Self::Testing => "Code health",
            Self::Security => "Risk reduction",
            Self::Delivery => "Velocity + reliability",
            Self::Collaboration => "Process maturity",
        }
    }

    /// All categories in report order.
    pub fn all() -> &'static [Category] {
        &[
            Self::Documentation,
            Self::LocalDevelopment,
            Self::Testing,
            Self::Security,
            Self::Delivery,
            Self::Collaboration,
        ]
    }
}

/// Observational status of a control.
///
/// Rules emit status based on evidence only; policy maps status to scores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Status {
    /// Control is present and appears actively enforced.
    Enforced,
    /// Control artifact is present.
    Present,
    /// Partial or weak evidence of the control.
    Partial,
    /// No evidence of the control.
    Missing,
    /// Control does not apply to this repository.
    NotApplicable,
    /// Insufficient signal to decide.
    Unknown,
}

impl Status {
    /// Whether this status indicates a gap worth recommending.
    pub fn is_gap(self) -> bool {
        matches!(self, Self::Missing | Self::Partial | Self::Unknown)
    }
}

/// Confidence in the observational finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// A single observable fact backing a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Relative path when the evidence is a file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Free-form observable detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Optional line number within `path`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

impl EvidenceItem {
    /// Evidence referencing a repository-relative path.
    pub fn path(path: impl Into<String>) -> Self {
        Self {
            path: Some(path.into()),
            detail: None,
            line: None,
        }
    }

    /// Evidence with a path and detail note.
    pub fn path_detail(path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            path: Some(path.into()),
            detail: Some(detail.into()),
            line: None,
        }
    }

    /// Detail-only evidence (no file path).
    pub fn detail(detail: impl Into<String>) -> Self {
        Self {
            path: None,
            detail: Some(detail.into()),
            line: None,
        }
    }
}

/// Normalized finding produced by a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Stable rule identifier, e.g. `documentation.readme`.
    pub rule: String,
    /// Hygiene category.
    pub category: Category,
    /// Observational status.
    pub status: Status,
    /// Confidence in the observation.
    pub confidence: Confidence,
    /// Short human-readable summary of what was observed.
    pub summary: String,
    /// Observable facts supporting the finding.
    pub evidence: Vec<EvidenceItem>,
    /// Optional remediation guidance (not a score judgment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

impl Finding {
    /// Start building a finding for a rule.
    pub fn builder(rule: impl Into<String>, category: Category) -> FindingBuilder {
        FindingBuilder {
            rule: rule.into(),
            category,
            status: Status::Unknown,
            confidence: Confidence::Medium,
            summary: String::new(),
            evidence: Vec::new(),
            remediation: None,
        }
    }
}

/// Fluent builder for [`Finding`].
#[derive(Debug)]
pub struct FindingBuilder {
    rule: String,
    category: Category,
    status: Status,
    confidence: Confidence,
    summary: String,
    evidence: Vec<EvidenceItem>,
    remediation: Option<String>,
}

impl FindingBuilder {
    pub fn status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    pub fn confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    pub fn evidence(mut self, items: Vec<EvidenceItem>) -> Self {
        self.evidence = items;
        self
    }

    pub fn push_evidence(mut self, item: EvidenceItem) -> Self {
        self.evidence.push(item);
        self
    }

    pub fn remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    pub fn build(self) -> Finding {
        Finding {
            rule: self.rule,
            category: self.category,
            status: self.status,
            confidence: self.confidence,
            summary: self.summary,
            evidence: self.evidence,
            remediation: self.remediation,
        }
    }
}
