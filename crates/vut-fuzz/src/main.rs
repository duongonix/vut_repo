fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iterations = std::env::args()
        .nth(1)
        .map_or(Ok(100_000), |value| value.parse::<usize>())?;
    let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let report = vut_fuzz::run(&corpus, iterations, 0x5655_545f_4655_5a5a)?;
    println!(
        "fuzzed {} corpus + {} generated cases ({} bytes)",
        report.corpus_cases, report.generated_cases, report.bytes
    );
    Ok(())
}
