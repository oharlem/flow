//! Build-time metadata: embed the short git SHA and build date as
//! `FLOW_GIT_SHA` / `FLOW_BUILD_DATE` so `flow --version` can show them.

use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=FLOW_GIT_SHA={sha}");
    println!("cargo:rustc-env=FLOW_BUILD_DATE={}", today_utc());

    println!("cargo:rerun-if-changed=build.rs");
    // Re-run when HEAD changes so the baked SHA stays fresh.
    let git_head = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".git")
        .join("HEAD");
    if git_head.exists() {
        println!("cargo:rerun-if-changed={}", git_head.display());
    }
}

/// Return today's date in ISO-8601 UTC (`YYYY-MM-DD`) computed from epoch
/// seconds. Avoids pulling in `chrono` as a build dependency.
fn today_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant's algorithm for days-since-epoch → (y, m, d) in the Gregorian
/// calendar. Correct for all dates the Rust CLI is ever going to see.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
