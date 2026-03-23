//! Markdown output formatting for FDA guidance documents.

use crate::types::FdaGuidanceDoc;

/// Format a single document as full Markdown detail.
pub fn format_detail(doc: &FdaGuidanceDoc) -> String {
    let mut out = format!("# {}\n\n", doc.title);
    out.push_str(&format!("**Status:** {}\n", doc.status));
    out.push_str(&format!("**Slug:** {}\n", doc.slug));

    if let Some(ref date) = doc.issue_date {
        out.push_str(&format!("**Issue Date:** {date}\n"));
    }

    if !doc.centers.is_empty() {
        out.push_str(&format!("**Centers:** {}\n", doc.centers.join(", ")));
    }
    if !doc.products.is_empty() {
        out.push_str(&format!("**Products:** {}\n", doc.products.join(", ")));
    }
    if !doc.topics.is_empty() {
        out.push_str(&format!("**Topics:** {}\n", doc.topics.join(", ")));
    }
    if !doc.document_type.is_empty() {
        out.push_str(&format!("**Type:** {}\n", doc.document_type));
    }

    out.push_str(&format!("\n**Page:** {}\n", doc.url));

    if let Some(ref pdf) = doc.pdf_url {
        out.push_str(&format!("**PDF:** {}", pdf));
        if let Some(ref size) = doc.pdf_size {
            out.push_str(&format!(" ({size})"));
        }
        out.push('\n');
    }

    if let Some(ref docket) = doc.docket_number {
        out.push_str(&format!("**Docket:** {docket}"));
        if let Some(ref url) = doc.docket_url {
            out.push_str(&format!(" ({url})"));
        }
        out.push('\n');
    }

    if doc.open_for_comment {
        out.push_str("**Open for Comment:** Yes");
        if let Some(ref close) = doc.comment_close_date {
            out.push_str(&format!(" (closes {close})"));
        }
        out.push('\n');
    }

    out
}

/// Format search results as a Markdown summary list.
pub fn format_search_results(docs: &[FdaGuidanceDoc], query: &str) -> String {
    if docs.is_empty() {
        return format!("No FDA guidance documents found matching '{query}'");
    }

    let mut out = format!(
        "Found {} FDA guidance document(s) matching '{query}':\n\n",
        docs.len()
    );

    for doc in docs {
        out.push_str(&format!("**{}**\n", doc.title));
        out.push_str(&format!(
            "  Status: {} | Date: {} | Centers: {}\n",
            doc.status,
            doc.issue_date.as_deref().unwrap_or("N/A"),
            if doc.centers.is_empty() {
                "N/A".to_string()
            } else {
                doc.centers.join(", ")
            },
        ));
        if let Some(ref pdf) = doc.pdf_url {
            out.push_str(&format!("  PDF: {pdf}\n"));
        }
        out.push_str(&format!("  Slug: {}\n\n", doc.slug));
    }

    out
}

/// Format categories as Markdown.
pub fn format_categories(
    by_center: &[(String, usize)],
    by_product: &[(String, usize)],
    by_topic: &[(String, usize)],
    total: usize,
) -> String {
    let mut out = format!("# FDA Guidance Document Categories ({total} documents)\n\n");

    out.push_str("## By FDA Center\n\n");
    for (name, count) in by_center {
        out.push_str(&format!("- **{name}**: {count} documents\n"));
    }

    out.push_str("\n## By Product Area\n\n");
    for (name, count) in by_product {
        out.push_str(&format!("- **{name}**: {count} documents\n"));
    }

    out.push_str("\n## By Topic (top 30)\n\n");
    for (name, count) in by_topic.iter().take(30) {
        out.push_str(&format!("- **{name}**: {count} documents\n"));
    }

    out
}

/// Format status-filtered results.
pub fn format_status_list(docs: &[FdaGuidanceDoc], status: &str, open_only: bool) -> String {
    let qualifier = if open_only {
        format!("{status} (open for comment)")
    } else {
        status.to_string()
    };

    if docs.is_empty() {
        return format!("No {qualifier} FDA guidance documents found.");
    }

    let mut out = format!(
        "# {} FDA Guidance Documents ({})\n\n",
        qualifier,
        docs.len()
    );

    for doc in docs {
        out.push_str(&format!("- **{}**\n", doc.title));
        out.push_str(&format!(
            "  Date: {} | Centers: {}\n",
            doc.issue_date.as_deref().unwrap_or("N/A"),
            doc.centers.join(", ")
        ));
        if doc.open_for_comment {
            if let Some(ref close) = doc.comment_close_date {
                out.push_str(&format!("  Comment closes: {close}\n"));
            }
        }
        out.push('\n');
    }

    out
}
