#![no_main]

use std::num::NonZeroUsize;
use std::time::Duration;

use boxpacker::compatibility::output_from_solution;
use boxpacker::model::{InputContainer, InputData, Item};
use boxpacker::report::render_html;
use boxpacker::solver::portfolio::PortfolioBackend;
use boxpacker::solver::{SolveRequest, SolverBackend};
use boxpacker::validate::{PackingInstance, validate_solution};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    let mut bytes = data.iter().copied().cycle();
    let container_count = usize::from(bytes.next().expect("cycled bytes are non-empty") % 5);
    let item_count = usize::from(bytes.next().expect("cycled bytes are non-empty") % 7);
    let containers = (0..container_count)
        .map(|index| InputContainer {
            name: format!("container-{index}"),
            width: dimension(&mut bytes),
            length: dimension(&mut bytes),
            height: dimension(&mut bytes),
        })
        .collect();
    let contents = (0..item_count)
        .map(|index| Item {
            name: format!("item-{index}"),
            width: dimension(&mut bytes),
            length: dimension(&mut bytes),
            height: dimension(&mut bytes),
        })
        .collect();
    let input = InputData {
        containers,
        contents,
    };
    let instance = PackingInstance::try_from(&input).expect("generated dimensions are exact");
    let seed = data
        .iter()
        .take(8)
        .enumerate()
        .fold(0_u64, |seed, (index, byte)| {
            seed | (u64::from(*byte) << (index * 8))
        });
    let request = SolveRequest::new(
        Duration::from_millis(10),
        seed,
        NonZeroUsize::new(1).expect("one is non-zero"),
    );
    let outcome = PortfolioBackend::new(NonZeroUsize::new(1).expect("one is non-zero"))
        .solve(&instance, &request)
        .expect("selected portfolio should solve a valid small instance");
    validate_solution(&instance, outcome.solution())
        .expect("selected portfolio result should validate independently");
    let output = output_from_solution(&instance, outcome.solution());
    serde_json::to_vec(&output).expect("compatible output should serialize");
    render_html(&output).expect("compatible output should render safely");
});

fn dimension(bytes: &mut impl Iterator<Item = u8>) -> f64 {
    f64::from(1 + bytes.next().expect("cycled bytes are non-empty") % 100) / 10.0
}
