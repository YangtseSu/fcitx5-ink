// SPDX-FileCopyrightText: 2005-2008 Taku Kudo
// SPDX-FileCopyrightText: 2026 Yangtse Su <yangtsesu@gmail.com>
// SPDX-License-Identifier: GPL-3.0-or-later AND BSD-3-Clause
//!
//! Offline port of zinnia's `zinnia_learn`: parse Zinnia S-expression
//! characters, extract the same sparse features as the recognizer, train one
//! linear SVM per class with the same coordinate descent, and write the
//! binary Zinnia model format directly.
//!
//! The SVM training loop is ported from Zinnia's `svm.cpp`
//! (<https://github.com/taku910/zinnia>), Copyright (C) 2008 Taku Kudo,
//! under the 3-clause BSD license; see `LICENSES/BSD-3-Clause.txt`.

use fcitx5_ink_features::extract_pairs;
use rayon::prelude::*;
use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

const MAGIC: u32 = 0x0EF7_1821;
const VERSION: u32 = 1;
const C: f64 = 1.0;
const EPS: f64 = 0.1;
const INF: f64 = 1e37;
const WEIGHT_THRESHOLD: f32 = 1.0e-3;
const LABEL_BYTES: usize = 16;

#[derive(Debug, Clone, PartialEq)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

fn tokenize(text: &str) -> Result<VecDeque<String>, String> {
    let mut tokens = VecDeque::new();
    let mut atom = String::new();
    for character in text.chars() {
        match character {
            '(' | ')' => {
                if !atom.is_empty() {
                    tokens.push_back(std::mem::take(&mut atom));
                }
                tokens.push_back(character.to_string());
            }
            c if c.is_whitespace() => {
                if !atom.is_empty() {
                    tokens.push_back(std::mem::take(&mut atom));
                }
            }
            c => atom.push(c),
        }
    }
    if !atom.is_empty() {
        tokens.push_back(atom);
    }
    Ok(tokens)
}

fn parse_sexp(tokens: &mut VecDeque<String>) -> Result<Sexp, String> {
    let token = tokens
        .pop_front()
        .ok_or_else(|| "unexpected end".to_owned())?;
    if token == "(" {
        let mut items = Vec::new();
        loop {
            let Some(next) = tokens.front() else {
                return Err("unclosed list".to_owned());
            };
            if next == ")" {
                tokens.pop_front();
                return Ok(Sexp::List(items));
            }
            items.push(parse_sexp(tokens)?);
        }
    }
    if token == ")" {
        return Err("unexpected )".to_owned());
    }
    Ok(Sexp::Atom(token))
}

fn list(value: &Sexp) -> Result<&[Sexp], String> {
    match value {
        Sexp::List(items) => Ok(items),
        Sexp::Atom(_) => Err("expected list".to_owned()),
    }
}

fn field<'a>(items: &'a [Sexp], name: &str) -> Option<&'a Sexp> {
    items.iter().find(|item| {
        matches!(item, Sexp::List(parts) if parts.first() == Some(&Sexp::Atom(name.to_owned())))
    })
}

fn atom(value: &Sexp) -> Result<&str, String> {
    match value {
        Sexp::Atom(value) => Ok(value),
        Sexp::List(_) => Err("expected atom".to_owned()),
    }
}

type ParsedCharacter = (String, Vec<Vec<(i32, i32)>>, u32);

fn parse_character(line: &str) -> Result<ParsedCharacter, String> {
    let mut tokens = tokenize(line)?;
    let root = parse_sexp(&mut tokens)?;
    if !tokens.is_empty() {
        return Err("trailing tokens".to_owned());
    }
    let items = list(&root)?;
    if items.first() != Some(&Sexp::Atom("character".to_owned())) {
        return Err("not a character".to_owned());
    }
    let value = field(items, "value")
        .and_then(|value| list(value).ok())
        .and_then(|parts| parts.get(1))
        .ok_or_else(|| "missing value".to_owned())?;
    let value = atom(value)?;
    if value.chars().count() != 1 {
        return Err("value must be one character".to_owned());
    }
    let width = field(items, "width")
        .and_then(|value| list(value).ok())
        .and_then(|parts| parts.get(1))
        .ok_or_else(|| "missing width".to_owned())?;
    let width = atom(width)?
        .parse::<u32>()
        .map_err(|_| "invalid width".to_owned())?;
    let height = field(items, "height")
        .and_then(|value| list(value).ok())
        .and_then(|parts| parts.get(1))
        .ok_or_else(|| "missing height".to_owned())?;
    let height = atom(height)?
        .parse::<u32>()
        .map_err(|_| "invalid height".to_owned())?;
    if width != height {
        return Err("only square canvases are supported".to_owned());
    }
    let strokes_node = field(items, "strokes")
        .and_then(|value| list(value).ok())
        .ok_or_else(|| "missing strokes".to_owned())?;
    let strokes = strokes_node
        .iter()
        .skip(1)
        .map(|stroke| {
            list(stroke)?
                .iter()
                .map(|point| {
                    let point = list(point)?;
                    if point.len() != 2 {
                        return Err("point needs x and y".to_owned());
                    }
                    let x = atom(&point[0])?
                        .parse::<f32>()
                        .map_err(|_| "invalid x".to_owned())?;
                    let y = atom(&point[1])?
                        .parse::<f32>()
                        .map_err(|_| "invalid y".to_owned())?;
                    Ok((x.round() as i32, y.round() as i32))
                })
                .collect::<Result<Vec<_>, String>>()
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((value.to_owned(), strokes, width))
}

#[derive(Clone)]
struct Example {
    label: String,
    features: Vec<(i32, f32)>,
}

struct ClassWeights {
    bias: f32,
    features: Vec<(i32, f32)>,
}

fn dot(weights: &[f64], features: &[(i32, f32)]) -> f64 {
    features
        .iter()
        .map(|(index, value)| weights[*index as usize] * f64::from(*value))
        .sum()
}

fn train_one(
    examples: &[Example],
    positive: &str,
    max_dim: usize,
    max_iterations: usize,
    seed: u64,
) -> ClassWeights {
    let mut weights = vec![0.0; max_dim + 1];
    let mut alpha = vec![0.0; examples.len()];
    let mut qd = vec![0.0; examples.len()];
    let mut index: Vec<usize> = (0..examples.len()).collect();
    let mut rng = seed | 1;
    for (i, example) in examples.iter().enumerate() {
        qd[i] = example
            .features
            .iter()
            .map(|(_, value)| f64::from(*value) * f64::from(*value))
            .sum();
    }
    let mut active_size = examples.len();
    let mut pgmax_old = INF;
    let mut pgmin_old = -INF;
    for _ in 0..max_iterations {
        let mut pgmax_new = -INF;
        let mut pgmin_new = INF;
        for i in (1..active_size).rev() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let j = (rng % (i as u64 + 1)) as usize;
            index.swap(i, j);
        }
        let mut s = 0;
        while s < active_size {
            let i = index[s];
            let y = if examples[i].label == positive {
                1.0
            } else {
                -1.0
            };
            let g = dot(&weights, &examples[i].features) * y - 1.0;
            let mut pg = 0.0;
            if alpha[i] == 0.0 {
                if g > pgmax_old {
                    active_size -= 1;
                    index.swap(s, active_size);
                    continue;
                } else if g < 0.0 {
                    pg = g;
                }
            } else if alpha[i] == C {
                if g < pgmin_old {
                    active_size -= 1;
                    index.swap(s, active_size);
                    continue;
                } else if g > 0.0 {
                    pg = g;
                }
            } else {
                pg = g;
            }
            pgmax_new = pgmax_new.max(pg);
            pgmin_new = pgmin_new.min(pg);
            if g.abs() > 1.0e-12 {
                let old = alpha[i];
                alpha[i] = (alpha[i] - g / qd[i]).clamp(0.0, C);
                let delta = (alpha[i] - old) * y;
                for (feature, value) in &examples[i].features {
                    weights[*feature as usize] += delta * f64::from(*value);
                }
            }
            s += 1;
        }
        if pgmax_new - pgmin_new <= EPS {
            if active_size == examples.len() {
                break;
            }
            active_size = examples.len();
            pgmax_old = INF;
            pgmin_old = -INF;
            continue;
        }
        pgmax_old = pgmax_new;
        pgmin_old = pgmin_new;
        if pgmax_old <= 0.0 {
            pgmax_old = INF;
        }
        if pgmin_old >= 0.0 {
            pgmin_old = -INF;
        }
    }
    ClassWeights {
        bias: weights[0] as f32,
        features: weights
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(index, value)| {
                (value.abs() >= f64::from(WEIGHT_THRESHOLD))
                    .then_some((index as i32, *value as f32))
            })
            .collect(),
    }
}

fn write_model(out: &Path, classes: &[String], weights: &[ClassWeights]) -> Result<(), String> {
    let mut bytes = vec![0u8; 12];
    for (label, weight) in classes.iter().zip(weights) {
        let mut label_bytes = [0u8; LABEL_BYTES];
        let source = label.as_bytes();
        if source.len() >= LABEL_BYTES {
            return Err(format!("label too long: {label}"));
        }
        label_bytes[..source.len()].copy_from_slice(source);
        bytes.extend_from_slice(&label_bytes);
        bytes.extend_from_slice(&weight.bias.to_le_bytes());
        for (index, value) in &weight.features {
            bytes.extend_from_slice(&index.to_le_bytes());
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&(-1i32).to_le_bytes());
        bytes.extend_from_slice(&0f32.to_le_bytes());
    }
    let size = bytes.len() as u32;
    bytes[0..4].copy_from_slice(&(size ^ MAGIC).to_le_bytes());
    bytes[4..8].copy_from_slice(&VERSION.to_le_bytes());
    bytes[8..12].copy_from_slice(&(classes.len() as u32).to_le_bytes());
    let mut file =
        fs::File::create(out).map_err(|error| format!("create {}: {error}", out.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("write {}: {error}", out.display()))?;
    Ok(())
}

struct Options {
    output: PathBuf,
    inputs: Vec<PathBuf>,
    max_iterations: usize,
    limit_classes: Option<usize>,
    jobs: Option<usize>,
    seed: u64,
}

fn train(options: &Options) -> Result<(), String> {
    let mut examples = Vec::new();
    for input in &options.inputs {
        let text = fs::read_to_string(input)
            .map_err(|error| format!("read {}: {error}", input.display()))?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (label, strokes, size) = parse_character(line)?;
            let Some(features) = extract_pairs(&strokes, size) else {
                continue;
            };
            examples.push(Example { label, features });
        }
    }
    let mut classes: BTreeSet<String> = examples.iter().map(|item| item.label.clone()).collect();
    if let Some(limit) = options.limit_classes {
        classes = classes.into_iter().take(limit).collect();
    }
    if examples.is_empty() || classes.is_empty() {
        return Err("no training examples".to_owned());
    }
    let max_dim = examples
        .iter()
        .flat_map(|item| item.features.iter().map(|(index, _)| *index))
        .max()
        .unwrap_or(0) as usize;
    let classes: Vec<String> = classes.into_iter().collect();
    println!(
        "{} examples, {} classes, max feature {}",
        examples.len(),
        classes.len(),
        max_dim
    );
    let mut builder = rayon::ThreadPoolBuilder::new();
    if let Some(jobs) = options.jobs {
        if jobs == 0 {
            return Err("--jobs must be greater than zero".to_owned());
        }
        builder = builder.num_threads(jobs);
    }
    let pool = builder
        .build()
        .map_err(|error| format!("build thread pool: {error}"))?;
    let completed = AtomicUsize::new(0);
    let started = Instant::now();
    let weights: Vec<ClassWeights> = pool.install(|| {
        classes
            .par_iter()
            .enumerate()
            .map(|(number, label)| {
                let weights = train_one(
                    &examples,
                    label,
                    max_dim,
                    options.max_iterations,
                    options.seed.wrapping_add(number as u64),
                );
                let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                if done.is_multiple_of(100) || done == classes.len() {
                    eprintln!(
                        "trained {done}/{} classes in {:.1}s",
                        classes.len(),
                        started.elapsed().as_secs_f64()
                    );
                }
                weights
            })
            .collect()
    });
    write_model(&options.output, &classes, &weights)?;
    println!(
        "wrote {} ({} classes, {:.1}s)",
        options.output.display(),
        classes.len(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

fn main() {
    let mut output = None;
    let mut inputs = Vec::new();
    let mut max_iterations = 2000;
    let mut limit_classes = None;
    let mut jobs = None;
    let mut seed = 1;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => output = Some(PathBuf::from(args.next().expect("--out needs a path"))),
            "--max-iterations" => {
                max_iterations = args
                    .next()
                    .expect("needs a number")
                    .parse()
                    .expect("number")
            }
            "--limit-classes" => {
                limit_classes = Some(
                    args.next()
                        .expect("needs a number")
                        .parse()
                        .expect("number"),
                )
            }
            "--jobs" => {
                jobs = Some(
                    args.next()
                        .expect("needs a number")
                        .parse()
                        .expect("number"),
                )
            }
            "--seed" => {
                seed = args
                    .next()
                    .expect("needs a number")
                    .parse()
                    .expect("number")
            }
            other if other.starts_with('-') => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
            other => inputs.push(PathBuf::from(other)),
        }
    }
    let Some(output) = output else {
        eprintln!("usage: ink-train --out MODEL [--max-iterations N] [--limit-classes N] [--jobs N] [--seed N] INPUT.s...");
        std::process::exit(2);
    };
    if inputs.is_empty() {
        eprintln!("no input files");
        std::process::exit(2);
    }
    if let Err(error) = train(&Options {
        output,
        inputs,
        max_iterations,
        limit_classes,
        jobs,
        seed,
    }) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = r#"
(character (value 一) (width 1000) (height 1000) (strokes ((100 500) (300 500) (500 500) (700 500) (900 500))))
(character (value 一) (width 1000) (height 1000) (strokes ((120 480) (320 490) (520 500) (720 510) (880 520))))
(character (value 丨) (width 1000) (height 1000) (strokes ((500 100) (500 300) (500 500) (500 700) (500 900))))
(character (value 丨) (width 1000) (height 1000) (strokes ((490 120) (495 320) (500 520) (505 720) (510 880))))
"#;

    #[test]
    fn trains_a_model_the_recognizer_can_load() {
        let directory = tempfile::tempdir().unwrap();
        let corpus = directory.path().join("train.s");
        let model = directory.path().join("model");
        fs::write(&corpus, CORPUS).unwrap();
        train(&Options {
            output: model.clone(),
            inputs: vec![corpus],
            max_iterations: 500,
            limit_classes: None,
            jobs: Some(1),
            seed: 1,
        })
        .unwrap();

        let horizontal = vec![vec![
            (100.0, 500.0),
            (300.0, 500.0),
            (500.0, 500.0),
            (700.0, 500.0),
            (900.0, 500.0),
        ]];
        let vertical = vec![vec![(500.0, 100.0), (500.0, 500.0), (500.0, 900.0)]];
        let h = fcitx5_ink_handwriting::handwriting_recognize(
            model.to_str().unwrap(),
            &horizontal,
            1000.0,
            1000.0,
        )
        .unwrap();
        let v = fcitx5_ink_handwriting::handwriting_recognize(
            model.to_str().unwrap(),
            &vertical,
            1000.0,
            1000.0,
        )
        .unwrap();
        assert_eq!(h.first().map(String::as_str), Some("一"));
        assert_eq!(v.first().map(String::as_str), Some("丨"));
    }

    #[test]
    fn parallel_training_is_deterministic() {
        let directory = tempfile::tempdir().unwrap();
        let corpus = directory.path().join("train.s");
        let serial = directory.path().join("serial.model");
        let parallel = directory.path().join("parallel.model");
        fs::write(&corpus, CORPUS).unwrap();
        for (output, jobs) in [(serial.clone(), 1), (parallel.clone(), 4)] {
            train(&Options {
                output,
                inputs: vec![corpus.clone()],
                max_iterations: 500,
                limit_classes: None,
                jobs: Some(jobs),
                seed: 1,
            })
            .unwrap();
        }
        assert_eq!(fs::read(serial).unwrap(), fs::read(parallel).unwrap());
    }
}
