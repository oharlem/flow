//! Regenerate `docs/SUMMARY.md` from the docs tree.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p flow-cli --example generate_summary \
//!   > docs/SUMMARY.md
//! ```
//!
//! The output is a single Markdown index with one section per docs area.

use flow_cli::summary;

fn main() {
    let repo = flow_core::paths::repo_root(None)
        .or_else(|_| std::env::current_dir().map_err(flow_core::Error::from))
        .expect("resolve repository root");
    print!("{}", summary::render_full(&repo));
}
