use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use boxpacker::model::InputData;
use boxpacker::objective::ObjectiveValue;
use boxpacker::solver::portfolio::PortfolioBackend;
use boxpacker::solver::{
    ProgressEvent, ProgressSink, ProgressWorkKind, SolveRequest, SolverBackend,
};
use boxpacker::validate::PackingInstance;

const CURRENT_INPUT: &str = include_str!("fixtures/current/input.json");

#[derive(Debug, Default)]
struct Recorder {
    events: Mutex<Vec<ProgressEvent>>,
}

impl ProgressSink for Recorder {
    fn record(&self, event: ProgressEvent) {
        self.events
            .lock()
            .expect("progress recorder lock should not be poisoned")
            .push(event);
    }
}

impl Recorder {
    fn events(&self) -> Vec<ProgressEvent> {
        self.events
            .lock()
            .expect("progress recorder lock should not be poisoned")
            .clone()
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum NormalizedEvent {
    PortfolioStarted {
        construction_work_units: usize,
        threads: usize,
        seed: u64,
    },
    WorkStarted {
        work_index: usize,
        kind: ProgressWorkKind,
    },
    CandidateValidated {
        work_index: usize,
        kind: ProgressWorkKind,
        objective: ObjectiveValue,
    },
    RepairFinished {
        work_index: usize,
        explored_nodes: u64,
        exhaustive: bool,
    },
    SolveFinished {
        explored_candidates: u64,
        validated_candidates: u64,
        improvements: u64,
        cancelled: bool,
    },
}

fn normalized(mut events: Vec<ProgressEvent>) -> Vec<NormalizedEvent> {
    let mut normalized = events
        .drain(..)
        .map(|event| match event {
            ProgressEvent::PortfolioStarted {
                construction_work_units,
                threads,
                seed,
            } => NormalizedEvent::PortfolioStarted {
                construction_work_units,
                threads: threads.get(),
                seed,
            },
            ProgressEvent::WorkStarted { work_index, kind } => {
                NormalizedEvent::WorkStarted { work_index, kind }
            }
            ProgressEvent::CandidateValidated {
                work_index,
                kind,
                objective,
            } => NormalizedEvent::CandidateValidated {
                work_index,
                kind,
                objective,
            },
            ProgressEvent::RepairFinished {
                work_index,
                explored_nodes,
                exhaustive,
            } => NormalizedEvent::RepairFinished {
                work_index,
                explored_nodes,
                exhaustive,
            },
            ProgressEvent::SolveFinished { metrics, cancelled } => NormalizedEvent::SolveFinished {
                explored_candidates: metrics.explored_candidates(),
                validated_candidates: metrics.validated_candidates(),
                improvements: metrics.improvements(),
                cancelled,
            },
        })
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized
}

#[test]
fn portfolio_emits_typed_progress_with_stable_work_identifiers() {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current fixture should deserialize");
    let instance = PackingInstance::try_from(&input).expect("current fixture should validate");
    let recorder = Arc::new(Recorder::default());
    let request = SolveRequest::new(
        Duration::from_secs(10),
        101,
        NonZeroUsize::new(4).expect("four is non-zero"),
    )
    .with_progress_sink(recorder.clone());

    let outcome = PortfolioBackend::default()
        .solve(&instance, &request)
        .expect("progress-enabled portfolio should solve");
    let events = recorder.events();

    assert_eq!(
        events.first(),
        Some(&ProgressEvent::PortfolioStarted {
            construction_work_units: 8,
            threads: NonZeroUsize::new(4).expect("four is non-zero"),
            seed: 101,
        })
    );
    assert_eq!(
        events.last(),
        Some(&ProgressEvent::SolveFinished {
            metrics: outcome.metrics(),
            cancelled: false,
        })
    );

    let started = events
        .iter()
        .filter_map(|event| match event {
            ProgressEvent::WorkStarted { work_index, kind } => Some((*work_index, *kind)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        started,
        BTreeMap::from([
            (0, ProgressWorkKind::Canonical),
            (1, ProgressWorkKind::Seeded),
            (2, ProgressWorkKind::Move),
            (3, ProgressWorkKind::Swap),
            (4, ProgressWorkKind::Rotation),
            (5, ProgressWorkKind::EjectionChain),
            (6, ProgressWorkKind::RuinRecreate),
            (7, ProgressWorkKind::Seeded),
            (8, ProgressWorkKind::ExactRepair),
        ])
    );

    let validated = events
        .iter()
        .filter_map(|event| match event {
            ProgressEvent::CandidateValidated {
                work_index,
                kind,
                objective,
            } => Some((*work_index, *kind, objective.unplaced_volume())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(validated.len(), 9);
    assert_eq!(
        validated
            .iter()
            .map(|(work_index, _, _)| *work_index)
            .collect::<std::collections::BTreeSet<_>>(),
        (0..=8).collect()
    );

    let repair = events
        .iter()
        .find_map(|event| match event {
            ProgressEvent::RepairFinished {
                work_index,
                explored_nodes,
                exhaustive,
            } => Some((*work_index, *explored_nodes, *exhaustive)),
            _ => None,
        })
        .expect("eligible residual should emit repair metrics");
    assert_eq!(repair.0, 8);
    assert!(repair.1 > 0);
}

#[test]
fn fixed_settings_reproduce_normalized_progress_and_aggregate_metrics() {
    let input: InputData =
        serde_json::from_str(CURRENT_INPUT).expect("current fixture should deserialize");
    let instance = PackingInstance::try_from(&input).expect("current fixture should validate");
    let backend = PortfolioBackend::default();

    let run = || {
        let recorder = Arc::new(Recorder::default());
        let request = SolveRequest::new(
            Duration::from_secs(10),
            149,
            NonZeroUsize::new(4).expect("four is non-zero"),
        )
        .with_progress_sink(recorder.clone());
        let outcome = backend
            .solve(&instance, &request)
            .expect("reproducible progress portfolio should solve");
        (outcome, normalized(recorder.events()))
    };

    let (first, first_events) = run();
    let (second, second_events) = run();

    assert_eq!(first.solution(), second.solution());
    assert_eq!(
        (
            first.metrics().explored_candidates(),
            first.metrics().validated_candidates(),
            first.metrics().improvements(),
        ),
        (
            second.metrics().explored_candidates(),
            second.metrics().validated_candidates(),
            second.metrics().improvements(),
        )
    );
    assert_eq!(first_events, second_events);
}
