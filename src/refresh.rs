//! Refresh pipeline: fetch FDA guidance JSON, parse HTML fields, write clean data.
//!
//! The FDA serves all 2,794 guidance documents as a single static JSON file at:
//! `https://www.fda.gov/files/api/datatables/static/search-for-guidance.json`
//!
//! Each record contains HTML-encoded string fields that must be parsed into
//! our clean `FdaGuidanceDoc` schema.

use crate::types::{FdaGuidanceDoc, FdaGuidanceError};
use regex::Regex;
use serde::Deserialize;

/// Raw record from FDA's static JSON (HTML-encoded fields)
#[derive(Debug, Deserialize)]
pub struct RawFdaRecord {
    /// Title wrapped in `<a href="...">Title</a>`
    pub title: String,
    /// PDF link or empty
    #[serde(rename = "field_associated_media_2")]
    pub media: String,
    /// Issue date as MM/DD/YYYY
    #[serde(rename = "field_issue_datetime")]
    pub issue_date: String,
    /// Issuing office(s), multi-org separated by `<br><br>`
    #[serde(rename = "field_issuing_office_taxonomy")]
    pub issuing_office: String,
    /// Comma-separated topic taxonomy
    #[serde(rename = "term_node_tid")]
    pub topics: String,
    /// "Draft" or "Final"
    #[serde(rename = "field_final_guidance_1")]
    pub status: String,
    /// "  Yes " or "  No " (whitespace-padded)
    #[serde(rename = "open-comment")]
    pub open_comment: String,
    /// Comment close date as MM/DD/YYYY or empty
    #[serde(rename = "field_comment_close_date")]
    pub comment_close_date: String,
    /// Docket number, may contain `<a>` link
    #[serde(rename = "field_docket_number")]
    pub docket_number: String,
    /// HTML-entity-encoded, comma-separated product areas
    #[serde(rename = "field_regulated_product_field")]
    pub products: String,
    /// Plain text document type
    #[serde(rename = "field_communication_type")]
    pub document_type: String,
    /// Plain text center name
    #[serde(rename = "field_center")]
    pub center: String,
    /// `<time datetime="...">...</time>` or empty
    #[serde(default)]
    pub changed: String,
}

/// Extract href and text from an `<a href="...">text</a>` fragment.
/// Returns `(href, text)` or `None` if no match.
pub fn extract_link(html: &str) -> Option<(String, String)> {
    let re = Regex::new(r#"<a\s+href="([^"]*)"[^>]*>([^<]*)"#).ok()?;
    let caps = re.captures(html)?;
    Some((caps[1].to_string(), caps[2].to_string()))
}

/// Extract ISO datetime from `<time datetime="...">` tag.
pub fn extract_time(html: &str) -> Option<String> {
    let re = Regex::new(r#"<time\s+datetime="([^"]*)"[^>]*>"#).ok()?;
    let caps = re.captures(html)?;
    Some(caps[1].to_string())
}

/// Parse MM/DD/YYYY to ISO 8601 YYYY-MM-DD.
/// Returns `None` for invalid or placeholder dates (year < 1901).
pub fn parse_us_date(date_str: &str) -> Option<String> {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let month = parts[0].parse::<u32>().ok()?;
    let day = parts[1].parse::<u32>().ok()?;
    let year = parts[2].parse::<u32>().ok()?;
    // Filter placeholders like 01/01/1900
    if year < 1901 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

/// Decode common HTML entities.
pub fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
}

/// Normalize center full name to abbreviation.
pub fn abbreviate_center(full_name: &str) -> String {
    match full_name.trim() {
        s if s.contains("Drug Evaluation and Research") => "CDER".to_string(),
        s if s.contains("Biologics Evaluation and Research") => "CBER".to_string(),
        s if s.contains("Devices and Radiological Health") => "CDRH".to_string(),
        s if s.contains("Food Safety and Applied Nutrition") => "CFSAN".to_string(),
        s if s.contains("Veterinary Medicine") => "CVM".to_string(),
        s if s.contains("Tobacco Products") => "CTP".to_string(),
        s if s.contains("Regulatory Affairs") => "ORA".to_string(),
        s if s.contains("Oncology Center of Excellence") => "OCE".to_string(),
        s if s.contains("Commissioner") => "OC".to_string(),
        other => other.trim().to_string(),
    }
}

/// Extract slug from a guidance page URL path.
///
/// Input: `/regulatory-information/search-fda-guidance-documents/some-slug`
/// Output: `some-slug`
pub fn extract_slug(url_path: &str) -> String {
    url_path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(url_path)
        .to_string()
}

/// Convert a raw FDA record to a clean `FdaGuidanceDoc`.
pub fn clean_record(raw: &RawFdaRecord) -> Result<FdaGuidanceDoc, FdaGuidanceError> {
    // Title + URL
    let (href, title_text) =
        extract_link(&raw.title).ok_or_else(|| FdaGuidanceError::HtmlParseFailed {
            field: "title".to_string(),
            reason: "No <a> tag found".to_string(),
        })?;

    let url = if href.starts_with('/') {
        format!("https://www.fda.gov{href}")
    } else {
        href.clone()
    };

    let slug = extract_slug(&href);

    // PDF
    let (pdf_url, pdf_size) = if raw.media.is_empty() {
        (None, None)
    } else {
        match extract_link(&raw.media) {
            Some((pdf_href, pdf_text)) => {
                let full_pdf = if pdf_href.starts_with('/') {
                    format!("https://www.fda.gov{pdf_href}")
                } else {
                    pdf_href
                };
                // Extract size from text like "PDF (291.05 KB)"
                let size = pdf_text.find('(').and_then(|start| {
                    pdf_text
                        .find(')')
                        .map(|end| pdf_text[start + 1..end].to_string())
                });
                (Some(full_pdf), size)
            }
            None => (None, None),
        }
    };

    // Issue date
    let issue_date = parse_us_date(&raw.issue_date);

    // Centers (split on <br><br>)
    let centers: Vec<String> = raw
        .issuing_office
        .split("<br><br>")
        .map(abbreviate_center)
        .filter(|s| !s.is_empty())
        .collect();

    // Topics
    let topics: Vec<String> = raw
        .topics
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Products (HTML-entity-encoded)
    let products: Vec<String> = decode_html_entities(&raw.products)
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Status
    let status = raw.status.trim().to_string();

    // Open for comment
    let open_for_comment = raw.open_comment.trim().eq_ignore_ascii_case("yes");

    // Comment close date
    let comment_close_date = parse_us_date(&raw.comment_close_date);

    // Docket
    let (docket_url, docket_number) = if raw.docket_number.is_empty() {
        (None, None)
    } else if raw.docket_number.contains('<') {
        match extract_link(&raw.docket_number) {
            Some((link_url, text)) => (Some(link_url), Some(text)),
            None => (None, Some(raw.docket_number.trim().to_string())),
        }
    } else {
        (None, Some(raw.docket_number.trim().to_string()))
    };

    // Document type
    let document_type = raw.document_type.trim().to_string();

    // Last modified
    let last_modified = extract_time(&raw.changed);

    Ok(FdaGuidanceDoc {
        slug,
        title: title_text.trim().to_string(),
        url,
        pdf_url,
        pdf_size,
        issue_date,
        status,
        centers,
        topics,
        products,
        docket_number,
        docket_url,
        open_for_comment,
        comment_close_date,
        document_type,
        last_modified,
    })
}

/// Parse raw FDA JSON array into clean documents.
pub fn parse_raw_json(json_str: &str) -> Result<Vec<FdaGuidanceDoc>, FdaGuidanceError> {
    let raw_records: Vec<RawFdaRecord> =
        serde_json::from_str(json_str).map_err(|e| FdaGuidanceError::ParseIndex(e.to_string()))?;

    let mut docs = Vec::with_capacity(raw_records.len());
    let mut errors = 0u32;

    for raw in &raw_records {
        match clean_record(raw) {
            Ok(doc) => docs.push(doc),
            Err(_) => errors += 1,
        }
    }

    // Sort by issue_date descending (newest first)
    docs.sort_by(|a, b| b.issue_date.cmp(&a.issue_date));

    if errors > 0 {
        eprintln!("Warning: {errors} records failed to parse");
    }

    Ok(docs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_link_basic() {
        let html = r#"<a href="/regulatory-information/search-fda-guidance-documents/my-doc">My Document Title</a>"#;
        let (href, text) = extract_link(html).unwrap_or_default();
        assert_eq!(
            href,
            "/regulatory-information/search-fda-guidance-documents/my-doc"
        );
        assert_eq!(text, "My Document Title");
    }

    #[test]
    fn test_extract_link_empty() {
        assert!(extract_link("").is_none());
        assert!(extract_link("plain text").is_none());
    }

    #[test]
    fn test_extract_time() {
        let html = r#"<time datetime="2024-10-01T07:00:53-04:00">2024-10-01 07:00</time>"#;
        assert_eq!(
            extract_time(html),
            Some("2024-10-01T07:00:53-04:00".to_string())
        );
    }

    #[test]
    fn test_parse_us_date_valid() {
        assert_eq!(parse_us_date("07/01/2001"), Some("2001-07-01".to_string()));
        assert_eq!(parse_us_date("12/31/2025"), Some("2025-12-31".to_string()));
    }

    #[test]
    fn test_parse_us_date_invalid() {
        assert_eq!(parse_us_date("01/01/1900"), None); // Placeholder
        assert_eq!(parse_us_date(""), None);
        assert_eq!(parse_us_date("not-a-date"), None);
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(
            decode_html_entities("Food &amp; Beverages"),
            "Food & Beverages"
        );
        assert_eq!(decode_html_entities("A &lt; B"), "A < B");
    }

    #[test]
    fn test_abbreviate_center() {
        assert_eq!(
            abbreviate_center("Center for Drug Evaluation and Research"),
            "CDER"
        );
        assert_eq!(
            abbreviate_center("Center for Biologics Evaluation and Research"),
            "CBER"
        );
        assert_eq!(
            abbreviate_center("Center for Devices and Radiological Health"),
            "CDRH"
        );
        assert_eq!(
            abbreviate_center("Center for Food Safety and Applied Nutrition"),
            "CFSAN"
        );
        assert_eq!(abbreviate_center("Center for Veterinary Medicine"), "CVM");
        assert_eq!(abbreviate_center("Center for Tobacco Products"), "CTP");
        assert_eq!(abbreviate_center("Office of Regulatory Affairs"), "ORA");
        assert_eq!(abbreviate_center("Unknown Office"), "Unknown Office");
    }

    #[test]
    fn test_extract_slug() {
        assert_eq!(
            extract_slug("/regulatory-information/search-fda-guidance-documents/my-guidance"),
            "my-guidance"
        );
        assert_eq!(extract_slug("/a/b/c/"), "c");
        assert_eq!(extract_slug("just-a-slug"), "just-a-slug");
    }

    #[test]
    fn test_extract_pdf_link() {
        let html = r#"<a href="/media/72074/download">PDF (291.05 KB)<span class="sr-only"> (my doc)</span></a>"#;
        let (href, text) = extract_link(html).unwrap_or_default();
        assert_eq!(href, "/media/72074/download");
        assert_eq!(text, "PDF (291.05 KB)");
    }

    #[test]
    fn test_clean_record_minimal() {
        let raw = RawFdaRecord {
            title: r#"<a href="/regulatory-information/search-fda-guidance-documents/test-doc">Test Document</a>"#.to_string(),
            media: String::new(),
            issue_date: "07/01/2024".to_string(),
            issuing_office: "Center for Drug Evaluation and Research".to_string(),
            topics: "ICH-Quality, Biosimilars".to_string(),
            status: "Final".to_string(),
            open_comment: "  No ".to_string(),
            comment_close_date: String::new(),
            docket_number: String::new(),
            products: "Drugs".to_string(),
            document_type: "Guidance Document".to_string(),
            center: "CDER".to_string(),
            changed: r#"<time datetime="2024-10-01T07:00:53-04:00">2024-10-01</time>"#.to_string(),
        };

        let doc = clean_record(&raw).unwrap_or_else(|e| {
            // Return a minimal doc for the assertion to fail with context
            FdaGuidanceDoc {
                slug: format!("ERROR: {e}"),
                title: String::new(),
                url: String::new(),
                pdf_url: None,
                pdf_size: None,
                issue_date: None,
                status: String::new(),
                centers: vec![],
                topics: vec![],
                products: vec![],
                docket_number: None,
                docket_url: None,
                open_for_comment: false,
                comment_close_date: None,
                document_type: String::new(),
                last_modified: None,
            }
        });
        assert_eq!(doc.slug, "test-doc");
        assert_eq!(doc.title, "Test Document");
        assert_eq!(
            doc.url,
            "https://www.fda.gov/regulatory-information/search-fda-guidance-documents/test-doc"
        );
        assert!(doc.pdf_url.is_none());
        assert_eq!(doc.issue_date, Some("2024-07-01".to_string()));
        assert_eq!(doc.status, "Final");
        assert_eq!(doc.centers, vec!["CDER"]);
        assert_eq!(doc.topics, vec!["ICH-Quality", "Biosimilars"]);
        assert!(!doc.open_for_comment);
        assert_eq!(
            doc.last_modified,
            Some("2024-10-01T07:00:53-04:00".to_string())
        );
    }
}
