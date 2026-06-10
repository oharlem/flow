//! `flow export-assets` — materialize embedded defaults for inspection.

use crate::args::ExportAssetsArgs;
use flow_core::{assets, Result};

/// Run `flow export-assets`.
pub fn run(args: ExportAssetsArgs) -> Result<()> {
    let conventions_dir = args.dir.join("conventions");
    std::fs::create_dir_all(&conventions_dir)?;
    for name in assets::CONVENTIONS_SHARD_NAMES {
        let body = assets::conventions_shard(name).unwrap_or_else(|| {
            panic!("embedded conventions shard missing: {name} — rebuild the binary")
        });
        std::fs::write(conventions_dir.join(format!("{name}.md")), body)?;
    }

    let agents_dir = args.dir.join("agents");
    std::fs::create_dir_all(&agents_dir)?;
    for phase in assets::PHASES {
        if let Some(body) = assets::agent_base(phase) {
            std::fs::write(agents_dir.join(format!("{phase}.base.md")), body)?;
        }
    }

    flow_core::logging::info(format!(
        "Exported embedded Flow assets to {}.",
        args.dir.display()
    ));
    Ok(())
}
