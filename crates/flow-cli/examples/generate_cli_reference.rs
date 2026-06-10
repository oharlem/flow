//! Regenerate `docs/reference/cli.md` from clap introspection.
//!
//! Run with:
//!
//! ```sh
//! cargo run -p flow-cli --example generate_cli_reference \
//!   > docs/reference/cli.md
//! ```
//!
//! The output is a single Markdown document with one section per subcommand.

use flow_cli::cli_help;

fn main() {
    print!("{}", cli_help::render_full());
}
