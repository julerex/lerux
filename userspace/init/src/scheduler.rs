use std::collections::VecDeque;

use crate::InitConfig;
use crate::unit::{Unit, UnitId, UnitKind, UnitStore};

pub struct Scheduler {
    pending: VecDeque<Job>,
}

struct Job {
    unit: UnitId,
    kind: JobKind,
    weak_dep_waits: usize,
}

enum JobKind {
    Start,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler {
            pending: VecDeque::new(),
        }
    }

    pub fn schedule_start_and_report_errors(
        &mut self,
        unit_store: &mut UnitStore,
        unit_id: UnitId,
    ) {
        let mut errors = vec![];
        self.schedule_start(unit_store, unit_id, &mut errors);
        for error in errors {
            eprintln!("init: {error}");
        }
    }

    pub fn schedule_start(
        &mut self,
        unit_store: &mut UnitStore,
        unit_id: UnitId,
        errors: &mut Vec<String>,
    ) {
        let loaded_units = unit_store.load_units(unit_id.clone(), errors);
        for unit_id in loaded_units {
            if !unit_store.unit(&unit_id).conditions_met() {
                continue;
            }

            self.pending.push_back(Job {
                unit: unit_id,
                kind: JobKind::Start,
                weak_dep_waits: 0,
            });
        }
    }

    pub fn step(&mut self, unit_store: &mut UnitStore, init_config: &mut InitConfig) {
        'a: loop {
            let Some(job) = self.pending.pop_front() else {
                return;
            };

            match job.kind {
                JobKind::Start => {
                    let unit = unit_store.unit_mut(&job.unit);

                    let mut waiting_on_weak_dep = false;
                    for dep in &unit.info.requires_weak {
                        for pending_job in &self.pending {
                            if &pending_job.unit == dep {
                                waiting_on_weak_dep = true;
                                break;
                            }
                        }
                        if waiting_on_weak_dep {
                            break;
                        }
                    }
                    if waiting_on_weak_dep {
                        let max_waits = self.pending.len().max(1);
                        if job.weak_dep_waits >= max_waits {
                            eprintln!(
                                "init: starting {} despite pending weak dependencies (possible cycle)",
                                job.unit.0
                            );
                        } else {
                            let mut job = job;
                            job.weak_dep_waits += 1;
                            self.pending.push_back(job);
                            continue 'a;
                        }
                    }

                    run(unit, init_config);
                }
            }
        }
    }
}

fn run(unit: &mut Unit, config: &mut InitConfig) {
    match &unit.kind {
        UnitKind::LegacyScript { script } => {
            for cmd in script.clone() {
                if config.log_debug {
                    eprintln!("init: running: {cmd:?}");
                }
                cmd.run(config);
            }
        }
        UnitKind::Service { service } => {
            if config.skip_cmd.contains(&service.cmd) {
                eprintln!("Skipping '{} {}'", service.cmd, service.args.join(" "));
                return;
            }
            if config.log_debug {
                eprintln!(
                    "Starting {} ({})",
                    unit.info.description.as_ref().unwrap_or(&unit.id.0),
                    service.cmd,
                );
            }
            service.spawn(&config.envs);
        }
        UnitKind::Target {} => {
            if config.log_debug {
                eprintln!(
                    "Reached target {}",
                    unit.info.description.as_ref().unwrap_or(&unit.id.0),
                );
            }
        }
    }
}
