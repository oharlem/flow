//! Property-style tests. Uses a minimal deterministic PRNG (no `proptest`
//! dependency) to exercise parsers against a broad input space.

use flow_core::ids::{FrId, MilestoneId, PId, RId, ScId, TaskId};
use flow_core::parse::{roadmap, status, tasks};
use std::str::FromStr;

// -- tiny deterministic RNG ---------------------------------------------------

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        // xorshift*
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next() as u32 % (hi - lo + 1))
    }
    fn bool(&mut self) -> bool {
        self.next() & 1 == 1
    }
    fn ascii_upper(&mut self) -> char {
        (b'A' + (self.next() as u8 % 26)) as char
    }
    fn ascii_lower(&mut self) -> char {
        (b'a' + (self.next() as u8 % 26)) as char
    }
}

fn random_fr_id(rng: &mut Rng) -> String {
    let mut s = String::from("FR-");
    if rng.bool() {
        s.push(rng.ascii_upper());
    }
    s.push_str(&rng.range(1, 9999).to_string());
    if rng.bool() {
        s.push(rng.ascii_lower());
    }
    s
}

fn random_sc_id(rng: &mut Rng) -> String {
    let mut s = String::from("SC-");
    if rng.bool() {
        s.push(rng.ascii_upper());
    }
    s.push_str(&rng.range(1, 9999).to_string());
    if rng.bool() {
        s.push(rng.ascii_lower());
    }
    s
}

fn random_task_id(rng: &mut Rng) -> String {
    let mut s = String::from("T-");
    if rng.bool() {
        s.push(rng.ascii_upper());
    }
    s.push_str(&rng.range(1, 9999).to_string());
    s
}

#[test]
fn prop_fr_id_round_trip() {
    let mut rng = Rng::new(0xD15EA5E_u64);
    for _ in 0..500 {
        let raw = random_fr_id(&mut rng);
        let id: FrId = raw.parse().expect("fr id parses");
        assert_eq!(id.to_string(), raw);
    }
}

#[test]
fn prop_sc_id_round_trip() {
    let mut rng = Rng::new(0xBADBEEF_u64);
    for _ in 0..500 {
        let raw = random_sc_id(&mut rng);
        let id: ScId = raw.parse().expect("sc id parses");
        assert_eq!(id.to_string(), raw);
    }
}

#[test]
fn prop_task_id_round_trip() {
    let mut rng = Rng::new(0xC0FFEE_u64);
    for _ in 0..500 {
        let raw = random_task_id(&mut rng);
        let id: TaskId = raw.parse().expect("task id parses");
        assert_eq!(id.to_string(), raw);
    }
}

#[test]
fn prop_milestone_id_round_trip() {
    for n in [1u32, 10, 100, 999, 1000, 9999] {
        let raw = format!("M-{n}");
        let id: MilestoneId = raw.parse().expect("m id parses");
        assert_eq!(id.to_string(), raw);
    }
}

#[test]
fn prop_p_r_ids_parse() {
    assert!("P-1".parse::<PId>().is_ok());
    assert!("P-9999".parse::<PId>().is_ok());
    assert!("R-V1".parse::<RId>().is_ok());
    assert!("R-001".parse::<RId>().is_ok());

    assert!(PId::from_str("P-10000").is_err()); // > 4 digits
    assert!(RId::from_str("r-001").is_err()); // lowercase prefix
}

#[test]
fn prop_status_parser_tolerates_extra_sections() {
    let text = "# Status: foo\n\n\
                **Change**: foo\n\
                **Started**: 2026-05-06\n\
                **Updated**: 2026-05-06T12:00:00Z\n\
                **State**: drafting\n\
                **Branch**: flow/foo\n\
                \n## History\n\n\
                - 2026-05-06T12:00:00Z — started — ok\n\
                \n## Known Regressions\n\n\
                - test_x — flaky on Windows (2026-05-06)\n\
                \n## Notes\n\n\
                Anything else here is preserved as raw text.\n";
    let s = status::parse_str(text);
    assert_eq!(s.feature, "foo");
    assert_eq!(s.history.len(), 1);
    // raw survives round-trip
    assert!(s.raw.contains("Known Regressions"));
    assert!(s.raw.contains("Anything else here is preserved"));
}

#[test]
fn prop_tasks_parser_handles_mixed_checkboxes() {
    let text = "## Tasks\n\n\
                - [ ] **T-001**: a\n  - Covers: FR-001\n\
                - [x] **T-002**: b\n  - Covers: FR-002, FR-003\n\
                - [X] **T-003**: c\n  - Depends-On: T-001, T-002\n\
                - [~] **T-004**: d\n  - Depends-On: T-003\n";
    let parsed = tasks::parse_str(text);
    assert_eq!(parsed.len(), 4);
    assert!(!parsed[0].done);
    assert!(parsed[1].done);
    assert!(parsed[2].done);
    assert!(!parsed[3].done);
    assert_eq!(parsed[0].state, tasks::TaskState::Open);
    assert_eq!(parsed[3].state, tasks::TaskState::AwaitingAcceptance);
    assert_eq!(parsed[1].covers, vec!["FR-002", "FR-003"]);
    assert_eq!(parsed[2].depends_on, vec!["T-001", "T-002"]);
}

#[test]
fn prop_roadmap_parser_handles_done_annotations() {
    let text = "## Milestones\n\n\
                ### [x] M-1: First\n\nFoo.\n\n\
                ### [ ] M-2: Next\n\nBar.\n\n\
                ### [x] M-3: Third\n";
    let ms = roadmap::parse_str(text);
    assert_eq!(ms.len(), 3);
    assert!(ms[0].done);
    assert_eq!(ms[0].title, "First");
    assert_eq!(ms[0].description.trim(), "Foo.");
    assert_eq!(ms[2].title, "Third");
    assert!(ms[2].description.is_empty());
}

#[test]
fn prop_status_stamp_preserves_existing_history() {
    let td = tempfile::TempDir::new().unwrap();
    let feat = td.path().join("f");
    std::fs::create_dir_all(&feat).unwrap();
    std::fs::write(
        feat.join("status.md"),
        "# Status: f\n\
         \n**Change**: f\n\
         **Started**: 2026-05-06\n\
         **Updated**: 2026-05-06T00:00:00Z\n\
         **State**: drafting\n\
         **Branch**: flow/f\n\
         \n## History\n\n\
         - 2026-05-06T00:00:00Z — started — seeded\n",
    )
    .unwrap();

    for i in 0..10 {
        let action = format!("action-{i}");
        status::stamp(&feat, Some(status::State::Building), &action, "ok").unwrap();
    }

    let s = status::parse_str(&std::fs::read_to_string(feat.join("status.md")).unwrap());
    // The 10 new entries + the initial "started" entry.
    assert_eq!(s.history.len(), 11);
    // Newest on top.
    assert!(s.history[0].action.starts_with("action-"));
    // Last entry is still the seed.
    assert_eq!(s.history.last().unwrap().action, "started");
    // State correctly updated.
    assert_eq!(s.state, Some(status::State::Building));
}
