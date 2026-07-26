use std::num::NonZeroUsize;
use std::time::Duration;

use boxpacker::model::InputData;
use boxpacker::solver::constructive::ConstructiveBackend;
use boxpacker::solver::{SolveRequest, SolverBackend};
use boxpacker::validate::PackingInstance;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

const CURRENT_INPUT: &str = include_str!("../tests/fixtures/current/input.json");
const SCALE_INPUT: &str = include_str!("../tests/fixtures/generated/scale_8x77.json");

fn instance(json: &str) -> PackingInstance {
    let input: InputData =
        serde_json::from_str(json).expect("benchmark fixture should deserialize");
    PackingInstance::try_from(&input).expect("benchmark fixture should validate")
}

fn backends() -> Vec<Box<dyn SolverBackend>> {
    vec![Box::new(ConstructiveBackend)]
}

fn request() -> SolveRequest {
    SolveRequest::new(
        Duration::from_secs(30),
        23,
        NonZeroUsize::new(1).expect("one is non-zero"),
    )
}

fn benchmark_fixture(criterion: &mut Criterion, fixture_name: &str, fixture: &PackingInstance) {
    let mut group = criterion.benchmark_group(fixture_name);
    group.throughput(Throughput::Elements(
        u64::try_from(fixture.items().len()).expect("fixture item count should fit u64"),
    ));
    for backend in backends() {
        group.bench_with_input(
            BenchmarkId::new("solve_and_validate", backend.name()),
            fixture,
            |bencher, instance| {
                bencher.iter(|| {
                    backend
                        .solve(instance, &request())
                        .expect("benchmark backend should solve")
                });
            },
        );
    }
    group.finish();
}

fn solver_benchmarks(criterion: &mut Criterion) {
    benchmark_fixture(criterion, "current_6x57", &instance(CURRENT_INPUT));
    benchmark_fixture(criterion, "scale_8x77", &instance(SCALE_INPUT));
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    targets = solver_benchmarks
}
criterion_main!(benches);
