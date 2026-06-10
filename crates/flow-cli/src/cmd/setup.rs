//! `flow setup` — alias for `flow init` that is idempotent and host-aware.

use crate::args::SetupArgs;
use flow_core::Result;

/// Run `flow setup`.
pub fn run(args: SetupArgs) -> Result<()> {
    crate::cmd::init::run(crate::args::InitArgs { host: args.host })
}
