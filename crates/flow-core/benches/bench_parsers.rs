//! Micro-benchmarks for the hot paths in `flow-core`.
//!
//! Run via `cargo bench -p flow-core`. The CI workflow does not run
//! benchmarks by default; they exist to catch major regressions during
//! performance work.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use flow_core::{drift, parse};

fn sample_spec() -> String {
    let mut s = String::from(
        "# Spec\n\n## What & Why\n\nBenchmark.\n\n## Requirements\n\n### Functional Requirements\n\n",
    );
    for i in 1..=200 {
        s.push_str(&format!("- **FR-{i:03}**: Requirement number {i}.\n"));
    }
    s.push_str("\n## Success Criteria\n\n### Measurable Outcomes\n\n");
    for i in 1..=100 {
        s.push_str(&format!("- **SC-{i:03}**: Outcome {i}.\n"));
    }
    s
}

fn sample_tasks(count: usize) -> String {
    let mut s = String::from("# Tasks\n\n## Tasks\n\n");
    for i in 1..=count {
        s.push_str(&format!(
            "- [ ] **T-{i:03}**: Task {i}.\n    - Covers: FR-{:03}\n    - Verifies: SC-{:03}\n",
            ((i - 1) % 200) + 1,
            ((i - 1) % 100) + 1,
        ));
    }
    s
}

fn bench_parse_spec_large(c: &mut Criterion) {
    let spec = sample_spec();
    c.bench_function("parse_spec_large", |b| {
        b.iter(|| {
            let parsed = parse::spec::parse_str(black_box(&spec));
            black_box(parsed);
        });
    });
}

fn bench_parse_tasks(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_tasks");
    for size in [100usize, 500, 1000] {
        let text = sample_tasks(size);
        group.bench_function(format!("tasks_{size}"), |b| {
            b.iter(|| {
                let parsed = parse::tasks::parse_str(black_box(&text));
                black_box(parsed);
            });
        });
    }
    group.finish();
}

fn bench_drift_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("drift_check");
    for size in [100usize, 500] {
        let td = tempfile::TempDir::new().unwrap();
        let feat = td.path().join(format!("f{size}"));
        std::fs::create_dir_all(&feat).unwrap();
        std::fs::write(feat.join("spec.md"), sample_spec()).unwrap();
        std::fs::write(feat.join("tasks.md"), sample_tasks(size)).unwrap();
        group.bench_function(format!("{size}_tasks"), |b| {
            b.iter(|| {
                let findings = drift::check_artifacts(black_box(&feat), None).unwrap();
                black_box(findings);
            });
        });
    }
    group.finish();
}

fn bench_render_drift(c: &mut Criterion) {
    let findings: Vec<drift::Finding> = (0..100)
        .map(|i| drift::Finding {
            id: "D1".into(),
            severity: drift::Severity::Warn,
            message: format!("FR 'FR-{i:03}' is defined in spec.md but not covered by any task"),
            title: "Requirement has no task".into(),
            cause: format!(
                "FR-{i:03} is in spec.md, but no task in tasks.md lists it under Covers."
            ),
            file: "spec.md".into(),
            line: Some(i),
            subject: format!("FR-{i:03}"),
            fix_options: vec![format!("Add a task in tasks.md with Covers: FR-{i:03}.")],
        })
        .collect();
    let report = drift::build_report(findings);
    c.bench_function("render_drift_100_findings", |b| {
        b.iter(|| {
            let out = drift::render::render(
                black_box(&report),
                drift::render::Mode::Status,
                "/flow-status",
                false,
            );
            black_box(out);
        });
    });
}

criterion_group!(
    benches,
    bench_parse_spec_large,
    bench_parse_tasks,
    bench_drift_check,
    bench_render_drift,
);
criterion_main!(benches);
