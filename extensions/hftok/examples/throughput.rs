//! Measure encode throughput over a JSON Lines prompt corpus.
//!
//! Usage: `throughput <tokenizer.json> <corpus.jsonl> [prompt-count]`
//!
//! Each line's `problem` field is one prompt. With a prompt count larger than
//! the corpus, the corpus is cycled, and the report says so, because a cycled
//! proxy is not the corpus it stands in for. Every prompt is also decoded and
//! compared with its NFC form, so a timing is never reported for wrong ids.

use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use mlpl_extension_hftok::{ByteLevelTokenizer, TokenizerFile};
use unicode_normalization::UnicodeNormalization;

fn prompts(path: &str) -> Result<Vec<String>, String> {
    let source = fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let row: serde_json::Value =
                serde_json::from_str(line).map_err(|error| format!("{path}: {error}"))?;
            row["problem"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{path}: a line has no string `problem` field"))
        })
        .collect()
}

fn run(arguments: &[String]) -> Result<(), String> {
    let [tokenizer_path, corpus_path, rest @ ..] = arguments else {
        return Err("usage: throughput <tokenizer.json> <corpus.jsonl> [prompt-count]".into());
    };
    let corpus = prompts(corpus_path)?;
    if corpus.is_empty() {
        return Err(format!("{corpus_path}: no prompts"));
    }
    let count = match rest.first() {
        Some(text) => text
            .parse::<usize>()
            .map_err(|error| format!("prompt count {text}: {error}"))?,
        None => corpus.len(),
    };

    let load_start = Instant::now();
    let source =
        fs::read_to_string(tokenizer_path).map_err(|error| format!("{tokenizer_path}: {error}"))?;
    let tokenizer = ByteLevelTokenizer::new(&TokenizerFile::parse(&source)?)?;
    let load = load_start.elapsed();

    let mut encoded = Vec::with_capacity(count);
    let encode_start = Instant::now();
    for prompt in corpus.iter().cycle().take(count) {
        encoded.push(tokenizer.encode(prompt)?);
    }
    let encode = encode_start.elapsed();

    let mut mismatches = 0;
    for (prompt, ids) in corpus.iter().cycle().zip(&encoded) {
        if tokenizer.decode(ids)? != prompt.nfc().collect::<String>() {
            mismatches += 1;
        }
    }
    let tokens: usize = encoded.iter().map(Vec::len).sum();
    let bytes: usize = corpus.iter().cycle().take(count).map(String::len).sum();

    println!(
        "corpus:            {corpus_path} ({} distinct prompts)",
        corpus.len()
    );
    println!(
        "prompts encoded:   {count}{}",
        if count > corpus.len() {
            " (corpus cycled)"
        } else {
            ""
        }
    );
    println!("input bytes:       {bytes}");
    println!("tokens produced:   {tokens}");
    println!("load ms:           {:.1}", load.as_secs_f64() * 1e3);
    println!("encode ms:         {:.1}", encode.as_secs_f64() * 1e3);
    let per_prompt = encode / u32::try_from(count).map_err(|error| error.to_string())?;
    println!("encode us/prompt:  {:.1}", per_prompt.as_secs_f64() * 1e6);
    println!("round-trip misses: {mismatches}");
    if mismatches > 0 {
        return Err(format!("{mismatches} prompts did not round-trip"));
    }
    Ok(())
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("throughput: {error}");
            ExitCode::FAILURE
        }
    }
}
