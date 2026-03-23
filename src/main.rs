#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
#![forbid(unsafe_code)]

//! FDA Guidance CLI — search, get, and refresh FDA guidance documents.
//! Build with: `cargo build -p nexcore-fda-guidance --features cli`

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};
#[cfg(feature = "cli")]
use std::path::PathBuf;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(
    name = "fda-guidance",
    about = "Search and manage FDA guidance documents"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// Refresh: fetch from FDA, clean HTML, write data/fda_guidance.json
    Refresh {
        /// Output path (default: data/fda_guidance.json relative to crate root)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Search guidance documents
    Search {
        /// Search query
        query: String,
        /// Filter by center (CDER, CBER, CDRH, etc.)
        #[arg(long)]
        center: Option<String>,
        /// Filter by product area
        #[arg(long)]
        product: Option<String>,
        /// Filter by status (Draft/Final)
        #[arg(long)]
        status: Option<String>,
        /// Max results (default: 10)
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Get a specific guidance document by slug
    Get {
        /// Document slug or partial title
        id: String,
    },
    /// List categories with document counts
    Categories,
    /// Get PDF URL for a guidance document
    Url {
        /// Document slug
        id: String,
    },
    /// List documents by status
    Status {
        /// "draft" or "final"
        status: String,
        /// Only show documents open for comment
        #[arg(long)]
        open_for_comment: bool,
    },
    /// Print dataset statistics
    Stats,
}

#[cfg(feature = "cli")]
const FDA_JSON_URL: &str =
    "https://www.fda.gov/files/api/datatables/static/search-for-guidance.json";

#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> nexcore_error::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Refresh { output } => {
            let out_path = output.unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/fda_guidance.json")
            });

            eprintln!("Fetching FDA guidance data from {FDA_JSON_URL}...");

            let client = reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; nexcore-fda-guidance/1.0)")
                .timeout(std::time::Duration::from_secs(60))
                .build()?;

            let response = client.get(FDA_JSON_URL).send().await?;
            let status_code = response.status();
            if !status_code.is_success() {
                nexcore_error::bail!("FDA returned HTTP {status_code}");
            }

            let raw_json = response.text().await?;
            eprintln!("Received {} bytes", raw_json.len());

            let docs = nexcore_fda_guidance::refresh::parse_raw_json(&raw_json)?;
            eprintln!("Parsed {} documents", docs.len());

            // Ensure output directory exists
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let json_output = serde_json::to_string_pretty(&docs)?;
            std::fs::write(&out_path, &json_output)?;
            eprintln!(
                "Wrote {} bytes to {}",
                json_output.len(),
                out_path.display()
            );

            // Stats
            let total = docs.len();
            let drafts = docs.iter().filter(|d| d.status == "Draft").count();
            let finals = docs.iter().filter(|d| d.status == "Final").count();
            let with_pdf = docs.iter().filter(|d| d.pdf_url.is_some()).count();
            let open_comment = docs.iter().filter(|d| d.open_for_comment).count();

            println!("FDA Guidance Refresh Complete:");
            println!("  Total:            {total}");
            println!("  Final:            {finals}");
            println!("  Draft:            {drafts}");
            println!("  With PDF:         {with_pdf}");
            println!("  Open for comment: {open_comment}");

            Ok(())
        }
        Commands::Search {
            query,
            center,
            product,
            status,
            limit,
        } => {
            let results = nexcore_fda_guidance::index::search(
                &query,
                center.as_deref(),
                product.as_deref(),
                status.as_deref(),
                limit,
            )?;
            print!(
                "{}",
                nexcore_fda_guidance::format::format_search_results(&results, &query)
            );
            Ok(())
        }
        Commands::Get { id } => {
            match nexcore_fda_guidance::index::get(&id)? {
                Some(doc) => print!("{}", nexcore_fda_guidance::format::format_detail(&doc)),
                None => eprintln!("Not found: {id}"),
            }
            Ok(())
        }
        Commands::Categories => {
            let docs = nexcore_fda_guidance::index::load_all()?;
            let by_center = nexcore_fda_guidance::index::categories_by_center(&docs);
            let by_product = nexcore_fda_guidance::index::categories_by_product(&docs);
            let by_topic = nexcore_fda_guidance::index::categories_by_topic(&docs);
            print!(
                "{}",
                nexcore_fda_guidance::format::format_categories(
                    &by_center,
                    &by_product,
                    &by_topic,
                    docs.len()
                )
            );
            Ok(())
        }
        Commands::Url { id } => {
            match nexcore_fda_guidance::index::get(&id)? {
                Some(doc) => match doc.pdf_url {
                    Some(ref pdf) => println!("{pdf}"),
                    None => println!("No PDF. View at: {}", doc.url),
                },
                None => eprintln!("Not found: {id}"),
            }
            Ok(())
        }
        Commands::Status {
            status,
            open_for_comment,
        } => {
            let docs = nexcore_fda_guidance::index::by_status(&status, open_for_comment)?;
            print!(
                "{}",
                nexcore_fda_guidance::format::format_status_list(&docs, &status, open_for_comment)
            );
            Ok(())
        }
        Commands::Stats => {
            let docs = nexcore_fda_guidance::index::load_all()?;
            let total = docs.len();
            let drafts = docs.iter().filter(|d| d.status == "Draft").count();
            let finals = docs.iter().filter(|d| d.status == "Final").count();
            let with_pdf = docs.iter().filter(|d| d.pdf_url.is_some()).count();
            let open = docs.iter().filter(|d| d.open_for_comment).count();
            let centers = nexcore_fda_guidance::index::categories_by_center(&docs);

            println!("FDA Guidance Document Statistics:");
            println!("  Total:            {total}");
            println!("  Final:            {finals}");
            println!("  Draft:            {drafts}");
            println!("  With PDF:         {with_pdf}");
            println!("  Open for comment: {open}");
            println!("\nBy Center:");
            for (name, count) in &centers {
                println!("  {name}: {count}");
            }
            Ok(())
        }
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("Build with --features cli to enable the FDA guidance CLI.");
}
