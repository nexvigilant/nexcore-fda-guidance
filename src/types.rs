//! FDA Guidance Document types
//!
//! Tier: T3 (Domain-specific FDA guidance record)

use serde::{Deserialize, Serialize};

/// A single FDA guidance document (cleaned from raw FDA HTML data).
///
/// Tier: T3 (Domain-specific)
/// Grounds to T1: String (∃), Vec (σ), Option (∃+∅), bool (κ)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FdaGuidanceDoc {
    /// URL slug serving as unique ID
    pub slug: String,
    /// Plain text title
    pub title: String,
    /// Full guidance page URL on fda.gov
    pub url: String,
    /// PDF download URL (absent for ~20% of older documents)
    #[serde(default)]
    pub pdf_url: Option<String>,
    /// PDF file size string (e.g., "291.05 KB")
    #[serde(default)]
    pub pdf_size: Option<String>,
    /// Issue date in ISO 8601 format (YYYY-MM-DD)
    #[serde(default)]
    pub issue_date: Option<String>,
    /// "Draft" or "Final"
    pub status: String,
    /// FDA centers (e.g., ["CDER", "CBER"])
    #[serde(default)]
    pub centers: Vec<String>,
    /// Topic taxonomy (e.g., ["ICH-Quality", "Biosimilars"])
    #[serde(default)]
    pub topics: Vec<String>,
    /// Regulated product areas (e.g., ["Drugs", "Biologics"])
    #[serde(default)]
    pub products: Vec<String>,
    /// Federal Register docket number
    #[serde(default)]
    pub docket_number: Option<String>,
    /// Regulations.gov docket URL
    #[serde(default)]
    pub docket_url: Option<String>,
    /// Whether the document is currently open for public comment
    #[serde(default)]
    pub open_for_comment: bool,
    /// Comment period closing date in ISO 8601 (YYYY-MM-DD)
    #[serde(default)]
    pub comment_close_date: Option<String>,
    /// Document type (e.g., "Guidance Document", "Compliance Policy Guide")
    #[serde(default)]
    pub document_type: String,
    /// Last modified timestamp in ISO 8601
    #[serde(default)]
    pub last_modified: Option<String>,
}

/// Errors from FDA guidance operations
#[derive(Debug, nexcore_error::Error)]
pub enum FdaGuidanceError {
    /// Failed to parse the embedded JSON index
    #[error("Failed to parse FDA guidance index: {0}")]
    ParseIndex(String),

    /// A required field was missing or malformed during refresh
    #[error("Invalid field '{field}': {reason}")]
    InvalidField { field: String, reason: String },

    /// HTTP fetch failed during refresh
    #[cfg(feature = "cli")]
    #[error("Failed to fetch FDA guidance data: {0}")]
    FetchFailed(String),

    /// HTML parsing failed during refresh
    #[error("HTML parse error in field '{field}': {reason}")]
    HtmlParseFailed { field: String, reason: String },
}
