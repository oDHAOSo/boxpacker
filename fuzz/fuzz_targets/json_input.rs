#![no_main]

use boxpacker::app::parse_input_document;
use boxpacker::validate::PackingInstance;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(document) = std::str::from_utf8(data)
        && let Ok(input) = parse_input_document(document)
    {
        let _ = PackingInstance::try_from(&input);
    }
});
