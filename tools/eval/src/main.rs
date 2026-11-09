// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later

use fcitx5_ink_core::model::Language;
use fcitx5_ink_core::recognize::{recognize, CANVAS};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Deserialize)]
struct Record {
    id: String,
    language: String,
    character: String,
    width: f32,
    height: f32,
    strokes: Vec<Vec<(f32, f32)>>,
    writer: Option<String>,
    source: Option<String>,
}

#[derive(Default)]
struct Counts {
    samples: u64,
    top1: u64,
    top5: u64,
    top12: u64,
    latencies: Vec<f64>,
    model_bytes: u64,
    failures: Vec<Failure>,
}

struct Failure {
    id: String,
    expected: String,
    candidates: Vec<String>,
}

fn language(value: &str) -> Option<Language> {
    Language::from_code(value)
}

fn model_path(models: &Path, language: Language) -> PathBuf {
    models.join(language.model_name())
}

fn scale(record: &Record) -> Vec<Vec<(f32, f32)>> {
    let sx = CANVAS / record.width;
    let sy = CANVAS / record.height;
    record
        .strokes
        .iter()
        .map(|stroke| {
            stroke
                .iter()
                .map(|(x, y)| {
                    (
                        x.clamp(0.0, record.width) * sx,
                        y.clamp(0.0, record.height) * sy,
                    )
                })
                .collect()
        })
        .collect()
}

fn main() {
    let mut data = PathBuf::from("reports/tomoe.jsonl");
    let mut models = PathBuf::from("models/zinnia");
    let mut only = None;
    let mut limit = None;
    let mut latency = false;
    let mut writer = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data" => data = PathBuf::from(args.next().expect("--data needs a path")),
            "--models" => models = PathBuf::from(args.next().expect("--models needs a path")),
            "--language" => only = Some(args.next().expect("--language needs a code")),
            "--limit" => {
                limit = Some(
                    args.next()
                        .expect("--limit needs a number")
                        .parse::<usize>()
                        .expect("--limit must be a number"),
                )
            }
            "--latency" => latency = true,
            "--writer" => writer = Some(args.next().expect("--writer needs an id")),
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let file = File::open(&data).unwrap_or_else(|error| {
        eprintln!("open {}: {error}; run scripts/tomoe_to_jsonl.py first", data.display());
        std::process::exit(1);
    });
    let mut totals: BTreeMap<&'static str, Counts> = BTreeMap::new();
    let mut captured = false;
    for line in BufReader::new(file).lines() {
        let line = line.expect("read JSONL");
        if line.trim().is_empty() {
            continue;
        }
        let record: Record = serde_json::from_str(&line).expect("parse JSONL");
        let Some(language) = language(&record.language) else {
            continue;
        };
        if let Some(filter) = writer.as_deref() {
            if record.writer.as_deref() != Some(filter) {
                continue;
            }
        }
        if record.source.as_deref() == Some("capture") {
            captured = true;
        }
        if let Some(filter) = only.as_deref() {
            if language.code() != filter {
                continue;
            }
        }
        let counts = totals.entry(language.code()).or_default();
        if let Some(limit) = limit {
            if counts.samples as usize >= limit {
                continue;
            }
        }
        counts.samples += 1;
        let model = model_path(&models, language);
        if counts.model_bytes == 0 {
            counts.model_bytes = std::fs::metadata(&model).map_or(0, |item| item.len());
        }
        let strokes = scale(&record);
        let started = Instant::now();
        let result = recognize(&model, &strokes, language);
        counts
            .latencies
            .push(started.elapsed().as_secs_f64() * 1000.0);
        match result {
            Ok(candidates) => {
                let position = candidates.iter().position(|item| item == &record.character);
                if position == Some(0) {
                    counts.top1 += 1;
                }
                if position.is_some_and(|index| index < 5) {
                    counts.top5 += 1;
                }
                if position.is_some_and(|index| index < 12) {
                    counts.top12 += 1;
                }
                if counts.failures.len() < 20 && position != Some(0) {
                    counts.failures.push(Failure {
                        id: record.id.clone(),
                        expected: record.character.clone(),
                        candidates: candidates.into_iter().take(5).collect(),
                    });
                }
            }
            Err(error) => {
                eprintln!("{}: {error}", record.id);
            }
        }
    }

    if latency {
        println!("| Language | Samples | Top-1 | Top-5 | Top-12 | Median ms | P95 ms | Model MB |");
        println!("|---|---:|---:|---:|---:|---:|---:|---:|");
    } else {
        println!("| Language | Samples | Top-1 | Top-5 | Top-12 |");
        println!("|---|---:|---:|---:|---:|");
    }
    for (language, counts) in &totals {
        let percent = |value: u64| {
            if counts.samples == 0 {
                0.0
            } else {
                value as f64 * 100.0 / counts.samples as f64
            }
        };
        if latency {
            let mut values = counts.latencies.clone();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = values.get(values.len() / 2).copied().unwrap_or(0.0);
            let p95 = values
                .get((values.len() * 95 / 100).min(values.len().saturating_sub(1)))
                .copied()
                .unwrap_or(0.0);
            println!(
                "| {language} | {} | {:.2}% | {:.2}% | {:.2}% | {:.2} | {:.2} | {:.1} |",
                counts.samples,
                percent(counts.top1),
                percent(counts.top5),
                percent(counts.top12),
                median,
                p95,
                counts.model_bytes as f64 / 1_000_000.0
            );
        } else {
            println!(
                "| {language} | {} | {:.2}% | {:.2}% | {:.2}% |",
                counts.samples,
                percent(counts.top1),
                percent(counts.top5),
                percent(counts.top12)
            );
        }
        for failure in &counts.failures {
            eprintln!(
                "  {language} {} expected {} got {}",
                failure.id,
                failure.expected,
                failure.candidates.join(" ")
            );
        }
    }
    if captured {
        eprintln!(
            "note: captured samples; keep the evaluation writer(s) separate from training"
        );
    } else {
        eprintln!(
            "note: these are Tomoe training samples, not a held-out generalization benchmark"
        );
    }
}
