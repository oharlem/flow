//! Embedded assets (templates, base prompts, conventions, gitignore fragments).
//!
//! These strings are baked into the binary via `include_str!` so Flow works
//! offline without any reference filesystem layout.

/// Return the raw text of a template file.
///
/// Known names (without path prefix):
/// `spec.md.tmpl`, `tasks.md.tmpl`, `status.md.tmpl`, `plan.md.tmpl`,
/// `roadmap.md.tmpl`, `run.md.tmpl`,
/// `run-log.md.tmpl`, `run-manual.md.tmpl`, `run-release-notes.md.tmpl`, `AGENTS.md.tmpl`,
/// `config.yaml.tmpl`.
#[must_use]
pub fn template(name: &str) -> Option<&'static str> {
    match name {
        "spec.md.tmpl" => Some(include_str!("../../../assets/templates/spec.md.tmpl")),
        "tasks.md.tmpl" => Some(include_str!("../../../assets/templates/tasks.md.tmpl")),
        "status.md.tmpl" => Some(include_str!("../../../assets/templates/status.md.tmpl")),
        "plan.md.tmpl" => Some(include_str!("../../../assets/templates/plan.md.tmpl")),
        "roadmap.md.tmpl" => Some(include_str!("../../../assets/templates/roadmap.md.tmpl")),
        "run.md.tmpl" => Some(include_str!("../../../assets/templates/run.md.tmpl")),
        "run-log.md.tmpl" => Some(include_str!("../../../assets/templates/run-log.md.tmpl")),
        "run-manual.md.tmpl" => Some(include_str!("../../../assets/templates/run-manual.md.tmpl")),
        "run-release-notes.md.tmpl" => Some(include_str!(
            "../../../assets/templates/run-release-notes.md.tmpl"
        )),
        "AGENTS.md.tmpl" => Some(include_str!("../../../assets/templates/AGENTS.md.tmpl")),
        "config.yaml.tmpl" => Some(include_str!("../../../assets/templates/config.yaml.tmpl")),
        _ => None,
    }
}

/// Return a base agent prompt by phase name (e.g. `"start"`, `"plan"`).
#[must_use]
pub fn agent_base(phase: &str) -> Option<&'static str> {
    match phase {
        "amend" => Some(include_str!("../../../assets/agents/amend.base.md")),
        "build" => Some(include_str!("../../../assets/agents/build.base.md")),
        "build-task" => Some(include_str!("../../../assets/agents/build-task.base.md")),
        "close" => Some(include_str!("../../../assets/agents/close.base.md")),
        "plan" => Some(include_str!("../../../assets/agents/plan.base.md")),
        "setup" => Some(include_str!("../../../assets/agents/setup.base.md")),
        "roadmap" => Some(include_str!("../../../assets/agents/roadmap.base.md")),
        "run" => Some(include_str!("../../../assets/agents/run.base.md")),
        "start" => Some(include_str!("../../../assets/agents/start.base.md")),
        "status" => Some(include_str!("../../../assets/agents/status.base.md")),
        "test" => Some(include_str!("../../../assets/agents/test.base.md")),
        _ => None,
    }
}

/// Canonical `assets/conventions/core.md` — the always-loaded shard containing
/// file rules, ID grammar, status schema, forbidden patterns, tolerance, and
/// confirmation behavior. Loaded on every Flow phase envelope.
pub const CONVENTIONS_CORE: &str = include_str!("../../../assets/conventions/core.md");

/// Version of the embedded artifact conventions bundled in this binary.
pub const CONVENTIONS_VERSION: &str = "1.1";

/// Spec-class shard — artifact shapes for `/flow-start` and `/flow-amend`.
pub const CONVENTIONS_SPEC: &str = include_str!("../../../assets/conventions/spec.md");

/// Plan-class shard — artifact shapes for `/flow-plan`.
pub const CONVENTIONS_PLAN: &str = include_str!("../../../assets/conventions/plan.md");

/// Build-class shard — task shape and task-state transitions for
/// `/flow-build` and `/flow-build-task`.
pub const CONVENTIONS_BUILD: &str = include_str!("../../../assets/conventions/build.md");

/// Test-class shard — verification behavior for `/flow-test`.
pub const CONVENTIONS_TEST: &str = include_str!("../../../assets/conventions/test.md");

/// Close-class shard — milestone shape and closeout augmentation contract for
/// `/flow-close`.
pub const CONVENTIONS_CLOSE: &str = include_str!("../../../assets/conventions/close.md");

/// Run-class shard — run artifact shapes and run behavior for `/flow-run`.
pub const CONVENTIONS_RUN: &str = include_str!("../../../assets/conventions/run.md");

/// Roadmap-class shard — source-preserving milestone shape for `/flow-roadmap`.
pub const CONVENTIONS_ROADMAP: &str = include_str!("../../../assets/conventions/roadmap.md");

/// Known conventions shard names. The order matches the canonical filesystem
/// layout under `.flow/conventions/`.
pub const CONVENTIONS_SHARD_NAMES: &[&str] = &[
    "core", "spec", "plan", "build", "test", "close", "run", "roadmap",
];

/// Return the embedded text of a conventions shard by bare name
/// (`"core"`, `"spec"`, `"plan"`, `"build"`, `"test"`, `"close"`, `"run"`,
/// `"roadmap"`).
///
/// Unknown names return `None`. The composer uses this embedded copy when an
/// on-disk shard file cannot be read.
#[must_use]
pub fn conventions_shard(name: &str) -> Option<&'static str> {
    match name {
        "core" => Some(CONVENTIONS_CORE),
        "spec" => Some(CONVENTIONS_SPEC),
        "plan" => Some(CONVENTIONS_PLAN),
        "build" => Some(CONVENTIONS_BUILD),
        "test" => Some(CONVENTIONS_TEST),
        "close" => Some(CONVENTIONS_CLOSE),
        "run" => Some(CONVENTIONS_RUN),
        "roadmap" => Some(CONVENTIONS_ROADMAP),
        _ => None,
    }
}

/// Bundled `.gitignore` fragment for the `.flow/` tree.
pub const FLOW_GITIGNORE: &str = include_str!("../../../assets/gitignore.d/flow.gitignore");

/// Names of supported phases that have both a `base.md` prompt and a CLI driver.
pub const PHASES: &[&str] = &[
    "roadmap",
    "run",
    "setup",
    "start",
    "amend",
    "plan",
    "build",
    "build-task",
    "test",
    "close",
    "status",
];

/// Metadata for a Flow command exposed through host adapters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostCommand {
    /// Host command name, without the `flow-` prefix.
    pub name: &'static str,
    /// CLI subcommand invoked after `flow`.
    pub flow_subcommand: &'static str,
    description: &'static str,
}

impl HostCommand {
    /// User-facing host command description for a generated adapter asset.
    #[must_use]
    pub fn description_for_host(self, host_label: &str) -> String {
        if self.name == "setup" {
            return format!("install or upgrade Flow for {host_label}");
        }
        self.description.to_string()
    }
}

/// Flow commands exposed through host adapters.
///
/// This intentionally includes utility commands, such as `doctor`, that do not
/// have phase base prompts under `.flow/agents/`.
pub const HOST_COMMANDS: &[HostCommand] = &[
    HostCommand {
        name: "roadmap",
        flow_subcommand: "roadmap",
        description: "decompose a PRD or notes file into Flow roadmap milestones",
    },
    HostCommand {
        name: "run",
        flow_subcommand: "run",
        description: "automate the active roadmap run — all open milestones, or one with `M-N`",
    },
    HostCommand {
        name: "setup",
        flow_subcommand: "setup",
        description: "",
    },
    HostCommand {
        name: "doctor",
        flow_subcommand: "doctor",
        description: "sanity-check the local Flow installation",
    },
    HostCommand {
        name: "start",
        flow_subcommand: "start",
        description: "draft a new change spec",
    },
    HostCommand {
        name: "amend",
        flow_subcommand: "amend",
        description: "update the active change spec",
    },
    HostCommand {
        name: "plan",
        flow_subcommand: "plan",
        description: "create an implementation plan and task list",
    },
    HostCommand {
        name: "build",
        flow_subcommand: "build",
        description: "implement all remaining tasks",
    },
    HostCommand {
        name: "build-task",
        flow_subcommand: "build-task",
        description: "implement one task",
    },
    HostCommand {
        name: "test",
        flow_subcommand: "test",
        description: "run build verification, tests, and consistency checks",
    },
    HostCommand {
        name: "close",
        flow_subcommand: "close",
        description: "close a completed change",
    },
    HostCommand {
        name: "status",
        flow_subcommand: "status",
        description: "show current status, consistency findings, and next action",
    },
];

/// Look up host command metadata by host command name.
#[must_use]
pub fn host_command(name: &str) -> Option<&'static HostCommand> {
    HOST_COMMANDS.iter().find(|command| command.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_phases_have_bases() {
        for phase in PHASES {
            assert!(agent_base(phase).is_some(), "missing base for {phase}");
        }
    }

    #[test]
    fn templates_are_non_empty() {
        for name in [
            "spec.md.tmpl",
            "tasks.md.tmpl",
            "status.md.tmpl",
            "plan.md.tmpl",
            "roadmap.md.tmpl",
            "run-log.md.tmpl",
            "run-manual.md.tmpl",
            "run-release-notes.md.tmpl",
            "AGENTS.md.tmpl",
            "config.yaml.tmpl",
        ] {
            assert!(
                template(name).is_some_and(|t| !t.is_empty()),
                "missing {name}"
            );
        }
    }

    #[test]
    fn conventions_core_contains_id_grammar() {
        assert_eq!(CONVENTIONS_VERSION, "1.1");
        assert!(CONVENTIONS_CORE.contains(&format!("Conventions-Version: {CONVENTIONS_VERSION}")));
        assert!(CONVENTIONS_CORE.contains("## 3. ID grammar"));
        assert!(CONVENTIONS_CORE.contains("## 9. Forbidden patterns"));
        assert!(CONVENTIONS_CORE.contains("## 11. Confirmation behavior"));
    }

    // T-001 / T-002 / T-003 / T-004 / T-005 / T-006 / T-007 / T-009:
    // every shard is embedded, non-empty, and carries the bumped version.
    #[test]
    fn t001_t009_every_shard_is_embedded_and_versioned() {
        for name in CONVENTIONS_SHARD_NAMES {
            let body = conventions_shard(name).unwrap_or_else(|| panic!("missing shard: {name}"));
            assert!(!body.is_empty(), "shard {name} is empty");
            assert!(
                body.contains(&format!("Conventions-Version: {CONVENTIONS_VERSION}")),
                "shard {name} missing version {CONVENTIONS_VERSION} header"
            );
        }
    }

    #[test]
    fn t002_spec_shard_holds_fr_and_sc_shapes() {
        assert!(CONVENTIONS_SPEC.contains("## 4.1 Functional Requirement"));
        assert!(CONVENTIONS_SPEC.contains("## 4.2 Success Criterion"));
        assert!(CONVENTIONS_SPEC.contains("## 7. The `## Clarifications` block"));
    }

    #[test]
    fn t003_plan_shard_holds_principle_and_cross_refs() {
        assert!(CONVENTIONS_PLAN.contains("## 4.6 Engineering Principle"));
        assert!(CONVENTIONS_PLAN.contains("## 5. Cross-references"));
        // Plan fix: the plan envelope is self-contained; §4.3 (shape only,
        // state transitions stay in build shard) is inlined here.
        assert!(CONVENTIONS_PLAN.contains("## 4.3 Task (shape)"));
    }

    #[test]
    fn t004_build_shard_holds_task_shape_and_state_transitions() {
        assert!(CONVENTIONS_BUILD.contains("## 4.3 Task"));
        assert!(CONVENTIONS_BUILD.contains("`[~]`"));
    }

    #[test]
    fn t006_close_shard_holds_close_behavior_and_milestone() {
        assert!(CONVENTIONS_CLOSE.contains("## 4.5 Milestone"));
        assert!(CONVENTIONS_CLOSE.contains("## 8. Close behavior"));
    }

    #[test]
    fn t007_run_shard_holds_run_behavior() {
        assert!(CONVENTIONS_RUN.contains("## Run behavior"));
        assert!(CONVENTIONS_RUN.contains("flow/runs/"));
    }

    #[test]
    fn t009_conventions_shard_dispatch_rejects_unknown() {
        assert!(conventions_shard("unknown-phase").is_none());
    }

    #[test]
    fn t002_roadmap_phase_registered() {
        assert!(agent_base("roadmap").is_some());
        assert!(PHASES.contains(&"roadmap"));
        assert!(host_command("roadmap").is_some());
    }

    #[test]
    fn t014_run_phase_registered() {
        assert!(agent_base("run").is_some());
        assert!(PHASES.contains(&"run"));
        assert!(host_command("run").is_some());
        assert!(host_command("run-all").is_none());
    }

    #[test]
    fn m8_run_prompt_keeps_child_start_retry_safety_net() {
        // T-002 / T-004: even after the green path receives FLOW_RUN_DIR
        // directly, the run agent keeps a retry path for synthetic child-start
        // failures.
        let body = agent_base("run").expect("run base prompt");
        assert!(body.contains("First child command"));
        assert!(body.contains("FLOW_RUN_DIR=<run-dir> flow start <M-N>"));
        assert!(body.contains("child-start-retry"));
        assert!(body.contains("retry once"));
    }

    #[test]
    fn t013_every_phase_prompt_has_canonical_confirmation_paragraph() {
        const CANONICAL_TEXT: &str = "When the runtime context contains **Confirmation**: disabled, skip the explicit \"Reply yes or y to save\" step";
        for phase in PHASES {
            let body = agent_base(phase).unwrap_or_else(|| panic!("no base for {phase}"));
            assert!(
                body.contains(CANONICAL_TEXT),
                "{phase}.base.md missing canonical confirmation paragraph"
            );
            assert!(
                body.contains("do not let it override **Confirmation**: disabled"),
                "{phase}.base.md missing disabled-confirmation precedence clause"
            );
        }
    }

    #[test]
    fn t003_roadmap_prompt_requires_preview_even_when_confirmation_is_disabled() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        assert!(
            body.contains("This preview step is required even when confirmation is disabled."),
            "roadmap prompt must keep preview separate from save confirmation"
        );
        assert!(
            body.contains("after showing the preview, write the roadmap file and run the printed finalize command directly"),
            "roadmap prompt must explain confirmation-disabled save behavior"
        );
    }

    #[test]
    fn t014_roadmap_prompt_requires_descriptor_heading() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        assert!(
            body.contains("Roadmap descriptor"),
            "roadmap prompt must name the descriptor the run command will reuse"
        );
        assert!(
            body.contains("# Roadmap: <Descriptor>"),
            "roadmap prompt must teach the descriptor-bearing H1 shape"
        );
    }

    #[test]
    fn t014_roadmap_prompt_describes_descriptor_source_and_generic_words() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        for expected in [
            "assignment summary",
            "source title",
            "first heading",
            "central noun phrase",
            "2–6 title-cased words",
            "`roadmap`",
            "`full`",
            "`feature`",
            "`project`",
            "`implementation`",
            "`build`",
            "`add`",
            "`fix`",
        ] {
            assert!(
                body.contains(expected),
                "roadmap prompt must include descriptor guidance for {expected:?}"
            );
        }
    }

    // M-1 T-001: five-field milestone shape labels appear in the roadmap prompt.
    #[test]
    fn m1_t001_roadmap_prompt_contains_five_field_milestone_labels() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        for label in [
            "Source:",
            "Outcome:",
            "Must preserve:",
            "Done when:",
            "Do not include:",
        ] {
            assert!(
                body.contains(label),
                "roadmap prompt must teach the `{label}` field label for the source-preserving milestone shape"
            );
        }
    }

    // M-1 T-002: the Core Rule sentence reaches the prompt verbatim so outcome
    // focus is preserved while detail is forced into the body.
    #[test]
    fn m1_t002_roadmap_prompt_contains_core_rule_sentence() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let expected = "each milestone must still be an outcome, not a task list, but its description must preserve enough source detail";
        assert!(
            body.contains(expected),
            "roadmap prompt must quote the Core Rule sentence so outcome focus is not traded away for detail"
        );
    }

    // M-1 T-003: the Milestone format example itself uses the new shape so the
    // agent has a concrete reference next to the instructions.
    #[test]
    fn m1_t003_roadmap_prompt_example_uses_five_field_shape() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let heading_pos = body
            .find("### [ ] M-1:")
            .expect("milestone example must include a `### [ ] M-1:` heading");
        // Inspect the 60-line slice after the heading for all five labels.
        let tail: String = body[heading_pos..]
            .lines()
            .take(60)
            .collect::<Vec<_>>()
            .join("\n");
        for label in [
            "Source:",
            "Outcome:",
            "Must preserve:",
            "Done when:",
            "Do not include:",
        ] {
            assert!(
                tail.contains(label),
                "milestone example under `### [ ] M-1:` must demonstrate the `{label}` field within the first 60 lines"
            );
        }
    }

    // M-2 T-001a: the prompt includes an explicit Coverage audit step.
    #[test]
    fn m2_t001a_roadmap_prompt_contains_coverage_audit_step() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("coverage audit") || lowered.contains("coverage map"),
            "roadmap prompt must describe a coverage-audit / coverage-map step before write"
        );
    }

    // M-2 T-001b: non-goals must be routed into `Do not include:`; manual
    // verification groups must be routed into `Done when:`.
    #[test]
    fn m2_t001b_roadmap_prompt_routes_non_goals_and_verification_into_five_fields() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("non-goal") && lowered.contains("do not include"),
            "roadmap prompt must map explicit non-goals into the milestone's `Do not include:` list"
        );
        assert!(
            lowered.contains("manual verification") && lowered.contains("done when"),
            "roadmap prompt must map manual verification groups into the milestone's `Done when:` list"
        );
    }

    // M-2 T-001c: unmappable sections must surface as a clarification question
    // or a dedicated milestone — never silent omission.
    #[test]
    fn m2_t001c_roadmap_prompt_requires_clarification_for_unmappable_sections() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("clarification question")
                || lowered.contains("clarifying question"),
            "roadmap prompt must instruct raising a clarification question when a section cannot be mapped"
        );
        assert!(
            lowered.contains("dedicated milestone"),
            "roadmap prompt must offer a dedicated milestone as the alternative to a clarification question"
        );
    }

    // M-2 T-001d: the roadmap must not be finalized until coverage is complete.
    #[test]
    fn m2_t001d_roadmap_prompt_blocks_finalize_until_coverage_complete() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("not finalized until coverage is complete")
                || lowered.contains("do not finalize the roadmap until coverage is complete")
                || lowered.contains("do not write the roadmap until coverage is complete"),
            "roadmap prompt must state that finalization is blocked until the coverage audit is complete"
        );
    }

    // M-2 T-001e: globally applicable criteria must be repeated across
    // milestones or captured in a final verification guardrail milestone.
    #[test]
    fn m2_t001e_roadmap_prompt_handles_global_criteria() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("repeat") && lowered.contains("every relevant milestone"),
            "roadmap prompt must allow repeating a global criterion in every relevant milestone"
        );
        assert!(
            lowered.contains("verification guardrail milestone")
                || lowered.contains("guardrail milestone"),
            "roadmap prompt must offer a final verification guardrail milestone for globally applicable criteria"
        );
    }

    // M-3 T-001a: cardinality preference (3–8) with the "large spec" qualifier.
    #[test]
    fn m3_t001a_roadmap_prompt_prefers_three_to_eight_milestones_on_large_spec() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        assert!(
            body.contains("3–8 milestones") || body.contains("3-8 milestones"),
            "roadmap prompt must state a preference for 3–8 milestones"
        );
        assert!(
            body.to_ascii_lowercase().contains("large spec"),
            "roadmap prompt must keep the `large spec` qualifier so small specs are not rejected"
        );
    }

    // M-3 T-001b: split by independently verifiable outcomes, not by code files.
    #[test]
    fn m3_t001b_roadmap_prompt_splits_by_outcome_not_by_code_files() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("independently verifiable"),
            "roadmap prompt must require splitting by independently verifiable outcomes"
        );
        assert!(
            lowered.contains("not by code files") || lowered.contains("not by code-files"),
            "roadmap prompt must explicitly prohibit splitting by code files"
        );
    }

    // M-3 T-001c: unrelated acceptance criteria must not be merged because they
    // share a screen.
    #[test]
    fn m3_t001c_roadmap_prompt_rejects_merging_unrelated_criteria_on_same_screen() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("unrelated acceptance criteria"),
            "roadmap prompt must name `unrelated acceptance criteria` as a thing not to merge"
        );
        assert!(
            lowered.contains("same screen"),
            "roadmap prompt must name the `same screen` anti-pattern"
        );
    }

    // M-3 T-001d: implementation-step milestones are prohibited; the prompt must
    // carry the source anti-example and positive example verbatim.
    #[test]
    fn m3_t001d_roadmap_prompt_contrasts_implementation_step_with_outcome_example() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        assert!(
            body.contains("update component CSS"),
            "roadmap prompt must quote the `update component CSS` anti-example so agents recognize the failure mode"
        );
        assert!(
            body.contains("Content cards match reference density and typography"),
            "roadmap prompt must quote the `Content cards match reference density and typography` positive example"
        );
    }

    // M-3 T-001e: every detailed source criterion must map to exactly one
    // milestone.
    #[test]
    fn m3_t001e_roadmap_prompt_requires_exactly_one_milestone_per_criterion() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("exactly one milestone"),
            "roadmap prompt must require every detailed acceptance criterion to map to `exactly one milestone`"
        );
    }

    // M-4 T-001a: the prompt enumerates all ten source signals.
    #[test]
    fn m4_t001a_roadmap_prompt_lists_ten_source_signals() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        for signal in [
            "user-facing outcomes",
            "required behavior",
            "acceptance criteria",
            "explicit non-goals",
            "source-of-truth files or screenshots",
            "design/token constraints",
            "sequencing dependencies",
            "verification requirements",
            "manual review requirements",
            "deferred work",
        ] {
            assert!(
                body.contains(signal),
                "roadmap prompt must enumerate the `{signal}` source signal in its ten-signal inventory"
            );
        }
    }

    // M-4 T-001b: the inventory is internal reasoning, not user-facing output.
    #[test]
    fn m4_t001b_roadmap_prompt_marks_inventory_as_internal_reasoning() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("internal reasoning")
                || lowered.contains("before writing milestones")
                || lowered.contains("before decomposition"),
            "roadmap prompt must state that the signal inventory is internal reasoning that runs before milestone writing"
        );
    }

    // M-4 T-001c: deferred work surfaced by the inventory lands in
    // `Do not include:` entries on the relevant milestones.
    #[test]
    fn m4_t001c_roadmap_prompt_routes_deferred_work_to_do_not_include() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("deferred work") && lowered.contains("do not include"),
            "roadmap prompt must route deferred work surfaced by the inventory into `Do not include:` entries"
        );
    }

    // M-4 T-001d: missing-signal cases must be handled without fabrication.
    #[test]
    fn m4_t001d_roadmap_prompt_forbids_fabrication_on_missing_signal() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("without fabricating")
                || lowered.contains("do not fabricate")
                || lowered.contains("never fabricate"),
            "roadmap prompt must forbid fabricating content when a source signal is absent"
        );
    }

    // M-5 T-001a: the prompt forbids vague "match the reference" phrasing and
    // requires substituting a precise anchor.
    #[test]
    fn m5_t001a_roadmap_prompt_forbids_match_the_reference() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        assert!(
            body.contains("match the reference"),
            "roadmap prompt must quote `match the reference` so the agent recognizes the anti-phrase"
        );
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("do not use")
                || lowered.contains("forbid")
                || lowered.contains("must not rely")
                || lowered.contains("never rely"),
            "roadmap prompt must forbid relying on `match the reference` phrasing"
        );
    }

    // M-5 T-001b: the prompt requires preservation of named literals (copy,
    // options, status labels, deferred columns, control behavior).
    #[test]
    fn m5_t001b_roadmap_prompt_preserves_named_literals() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        for literal in [
            "copy strings",
            "option names",
            "status labels",
            "deferred column names",
            "required control behavior",
        ] {
            assert!(
                body.contains(literal),
                "roadmap prompt must require preservation of `{literal}` from the source document"
            );
        }
    }

    // M-5 T-001c: the three canonical implementation-constraint examples are
    // quoted verbatim from the draft.
    #[test]
    fn m5_t001c_roadmap_prompt_quotes_three_canonical_constraint_examples() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        for example in [
            "no raw Tailwind status colors",
            "no SEO column",
            "minimum 14px text",
        ] {
            assert!(
                body.contains(example),
                "roadmap prompt must quote the `{example}` constraint example verbatim from the draft"
            );
        }
    }

    // M-5 T-001d: the prompt names roadmap automation as the reason literal
    // preservation is mandatory.
    #[test]
    fn m5_t001d_roadmap_prompt_cites_flow_run_reason_for_literal_preservation() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        assert!(
            body.contains("/flow-run") || body.contains("`flow run`"),
            "roadmap prompt must name roadmap automation as the reason literal preservation is mandatory"
        );
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("only sees") || lowered.contains("does not reread"),
            "roadmap prompt must explain that roadmap automation only sees the milestone text"
        );
    }

    // M-6 T-001a: the six new negative-constraint guardrails are present.
    #[test]
    fn m6_t001a_roadmap_prompt_extends_must_not_block_with_six_new_guardrails() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        let lowered = body.to_ascii_lowercase();
        assert!(
            lowered.contains("do not collapse")
                && (lowered.contains("one-paragraph") || lowered.contains("one paragraph")),
            "roadmap prompt must forbid collapsing detailed PRDs into one-paragraph milestones"
        );
        assert!(
            lowered.contains("do not omit non-goals"),
            "roadmap prompt must forbid omitting non-goals"
        );
        assert!(
            lowered.contains("do not omit verification criteria"),
            "roadmap prompt must forbid omitting verification criteria"
        );
        assert!(
            lowered.contains("do not invent new requirements"),
            "roadmap prompt must forbid inventing new requirements"
        );
        assert!(
            lowered.contains("do not turn the roadmap into") && lowered.contains("tasks.md"),
            "roadmap prompt must forbid turning the roadmap into `tasks.md`"
        );
        assert!(
            lowered.contains("do not make milestones so thin")
                && lowered.contains("memory of the original prompt"),
            "roadmap prompt must forbid memory-dependent thin milestones"
        );
    }

    // M-6 T-001b: the preexisting guardrails in the must-not block survive.
    #[test]
    fn m6_t001b_roadmap_prompt_preserves_existing_guardrails() {
        let body = agent_base("roadmap").expect("roadmap base prompt");
        for existing in [
            "Do not modify existing milestone IDs, titles, or descriptions in append mode.",
            "Do not write `plan.md`, `tasks.md`, or `status.md`.",
            "Do not run git commands.",
            "Do not ask more than 3 clarifying questions.",
        ] {
            assert!(
                body.contains(existing),
                "roadmap prompt must keep the existing guardrail `{existing}`"
            );
        }
    }

    // M-7 T-002a: the dedicated roadmap shard is registered and embedded.
    #[test]
    fn m7_t002a_roadmap_shard_is_registered_and_embedded() {
        let body = conventions_shard("roadmap")
            .expect("conventions_shard(\"roadmap\") must return Some once the shard is registered");
        assert!(!body.is_empty(), "roadmap shard must not be empty");
        assert!(
            body.contains("Conventions-Version: 1.1"),
            "roadmap shard must carry the `Conventions-Version: 1.1` header"
        );
        for label in [
            "Source:",
            "Outcome:",
            "Must preserve:",
            "Done when:",
            "Do not include:",
        ] {
            assert!(
                body.contains(label),
                "roadmap shard must document the `{label}` field label"
            );
        }
    }

    // M-7 T-002b: the shard name is included in the canonical shard-name list so
    // any tooling that iterates shard names picks it up.
    #[test]
    fn m7_t002b_roadmap_shard_is_in_canonical_names_list() {
        assert!(
            CONVENTIONS_SHARD_NAMES.contains(&"roadmap"),
            "CONVENTIONS_SHARD_NAMES must include `roadmap` so iterators pick up the new shard"
        );
    }

    // M-7 T-005: the roadmap template reflects the five-field shape so human
    // authors see the expected structure in the seeded file.
    #[test]
    fn m7_t005_roadmap_template_shows_five_field_shape() {
        let tmpl =
            template("roadmap.md.tmpl").expect("roadmap.md.tmpl must resolve to embedded text");
        for label in [
            "Source:",
            "Outcome:",
            "Must preserve:",
            "Done when:",
            "Do not include:",
        ] {
            assert!(
                tmpl.contains(label),
                "roadmap.md.tmpl must show the `{label}` field in its example comment block"
            );
        }
    }
}
