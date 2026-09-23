//! Optimization pass manager.
//!
//! Groups passes into phases and runs them with a bounded fixpoint. After each
//! phase the program is verified; a phase that breaks an invariant is reverted
//! to its input, so a buggy pass degrades to no optimization for that phase
//! instead of miscompiling. The whole batch is additionally verified by
//! [`super::optimize`], which reverts everything if needed.

use super::{OptimizationLevel, OptimizationReport, verify};
use crate::Program;

type PassFn = fn(&mut Program) -> OptimizationReport;

struct Phase {
    passes: Vec<PassFn>,
}

impl Phase {
    fn new(passes: Vec<PassFn>) -> Self {
        Self { passes }
    }
}

pub struct PassManager {
    phases: Vec<Phase>,
    rounds: u32,
}

impl PassManager {
    #[must_use]
    pub fn for_level(level: OptimizationLevel) -> Self {
        let mut phases = vec![
            Phase::new(vec![super::inline::run]),
            Phase::new(vec![
                super::sccp::run,
                super::strings::run,
                super::cse::run,
                super::closure::run,
                super::branch::run,
                super::copy_prop::run,
                super::dce::run,
            ]),
            Phase::new(vec![
                super::move_prop::run,
                super::rc::run,
                super::drop::run,
                super::memory::run,
                super::dce::run,
            ]),
            Phase::new(vec![
                super::copy_elision::run,
                super::sroa::run,
                super::loops::run,
            ]),
            Phase::new(vec![
                super::collections::run,
                super::bounds::run,
                super::async_opt::run,
                super::dce::run,
                super::branch::run,
            ]),
        ];
        if level >= OptimizationLevel::O3 {
            phases.push(Phase::new(vec![
                super::sccp::run,
                super::copy_prop::run,
                super::rc::run,
                super::dce::run,
            ]));
        }
        Self { phases, rounds: 1 }
    }

    pub fn run(&self, program: &mut Program, report: &mut OptimizationReport) {
        for _ in 0..self.rounds {
            let mut round_changed = false;
            for phase in &self.phases {
                if run_phase(phase, program, report) {
                    round_changed = true;
                }
            }
            if !round_changed {
                break;
            }
        }
    }
}

fn run_phase(phase: &Phase, program: &mut Program, report: &mut OptimizationReport) -> bool {
    let program_backup = program.clone();
    let report_backup = *report;
    let mut changed = false;
    for pass in &phase.passes {
        let delta = pass(program);
        if delta != OptimizationReport::default() {
            changed = true;
        }
        report.merge(delta);
    }
    if verify(program).is_err() {
        *program = program_backup;
        *report = report_backup;
        report.reverted_phases += 1;
        return false;
    }
    changed
}
