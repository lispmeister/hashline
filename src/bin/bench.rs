use hashline::edit::SetLineOp;
use hashline::HashlineEdit;
use hashline::{apply_hashline_edits, compute_line_hash, format_hashlines, parse_line_ref};
use std::time::Instant;

fn generate_file(num_lines: usize) -> String {
    (0..num_lines)
        .map(|i| format!("    let var_{} = compute_something({}, \"arg\");", i, i * 7))
        .collect::<Vec<_>>()
        .join("\n")
}

fn bench<F: FnMut()>(name: &str, iterations: usize, mut f: F) -> f64 {
    // Warmup
    for _ in 0..3 {
        f();
    }
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed();
    let per_iter_us = elapsed.as_secs_f64() * 1_000_000.0 / iterations as f64;
    let _ = name;
    per_iter_us
}

// --- JSON output types ---

#[derive(serde::Serialize)]
struct BenchResult {
    benchmark: String,
    file_lines: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    edit_count: Option<usize>,
    metric: &'static str,
    value: f64,
}

#[derive(serde::Serialize)]
struct BenchReport {
    version: String,
    commit: String,
    timestamp: String,
    runner: String,
    results: Vec<BenchResult>,
}

fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn iso_timestamp() -> String {
    // RFC3339 without external deps: use std::time
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as YYYY-MM-DDTHH:MM:SSZ
    let s = secs;
    let (y, mo, d, h, mi, sec) = {
        let mut days = s / 86400;
        let time = s % 86400;
        let h = time / 3600;
        let mi = (time % 3600) / 60;
        let sec = time % 60;
        // Gregorian calendar from days since epoch
        let mut y = 1970u64;
        loop {
            let leap = y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
            let ydays = if leap { 366 } else { 365 };
            if days < ydays {
                break;
            }
            days -= ydays;
            y += 1;
        }
        let leap = y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
        let month_days = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        let mut mo = 1u64;
        for &md in &month_days {
            if days < md {
                break;
            }
            days -= md;
            mo += 1;
        }
        (y, mo, days + 1, h, mi, sec)
    };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, mi, sec)
}

fn run_benchmarks() -> Vec<BenchResult> {
    let sizes = [100, 1_000, 10_000];
    let mut results = Vec::new();

    // --- format_hashlines ---
    for &size in &sizes {
        let content = generate_file(size);
        let us = bench("format_hashlines", 50, || {
            let _ = format_hashlines(&content, 1);
        });
        results.push(BenchResult {
            benchmark: "format_hashlines".to_string(),
            file_lines: size,
            edit_count: None,
            metric: "us_per_iter",
            value: (us * 10.0).round() / 10.0,
        });
    }

    // --- compute_line_hash ---
    for &size in &sizes {
        let content = generate_file(size);
        let lines: Vec<&str> = content.split('\n').collect();
        let us = bench("compute_line_hash", 50, || {
            for (i, line) in lines.iter().enumerate() {
                let _ = compute_line_hash(i + 1, line);
            }
        });
        results.push(BenchResult {
            benchmark: "compute_line_hash".to_string(),
            file_lines: size,
            edit_count: None,
            metric: "us_per_iter",
            value: (us * 10.0).round() / 10.0,
        });
    }

    // --- parse_line_ref ---
    let refs = ["1:ab", "100:ff", "9999:0a", "42:de|some content here"];
    let iters = 10_000;
    let us = bench("parse_line_ref", 100, || {
        for _ in 0..iters {
            for r in &refs {
                let _ = parse_line_ref(r);
            }
        }
    });
    let calls = iters * refs.len();
    results.push(BenchResult {
        benchmark: "parse_line_ref".to_string(),
        file_lines: 0,
        edit_count: Some(calls),
        metric: "us_per_iter",
        value: (us * 10.0).round() / 10.0,
    });

    // --- apply_hashline_edits ---
    for &size in &sizes {
        let content = generate_file(size);
        let lines: Vec<&str> = content.split('\n').collect();

        for &num_edits in &[1usize, 5, 20] {
            if num_edits > size {
                continue;
            }
            let edits: Vec<HashlineEdit> = (0..num_edits)
                .map(|i| {
                    let line_idx = (i * size / num_edits).min(size - 1);
                    let line_num = line_idx + 1;
                    let hash = compute_line_hash(line_num, lines[line_idx]);
                    HashlineEdit::SetLine {
                        set_line: SetLineOp {
                            anchor: format!("{}:{}", line_num, hash),
                            new_text: format!("    let var_{} = REPLACED;", line_idx),
                        },
                    }
                })
                .collect();

            let us = bench("apply_hashline_edits", 50, || {
                let _ = apply_hashline_edits(&content, &edits);
            });
            results.push(BenchResult {
                benchmark: "apply_hashline_edits".to_string(),
                file_lines: size,
                edit_count: Some(num_edits),
                metric: "us_per_iter",
                value: (us * 10.0).round() / 10.0,
            });
        }
    }

    // --- batched edits on 1k-line file ---
    let mid_content = generate_file(1_000);
    let mid_lines: Vec<&str> = mid_content.split('\n').collect();

    for &num_edits in &[1usize, 10, 50, 100] {
        let edits: Vec<HashlineEdit> = (0..num_edits)
            .map(|i| {
                let line_idx = (i * 1_000 / num_edits).min(999);
                let line_num = line_idx + 1;
                let hash = compute_line_hash(line_num, mid_lines[line_idx]);
                HashlineEdit::SetLine {
                    set_line: SetLineOp {
                        anchor: format!("{}:{}", line_num, hash),
                        new_text: format!("    let var_{} = REPLACED;", line_idx),
                    },
                }
            })
            .collect();

        let iters = if num_edits >= 50 { 20 } else { 50 };
        let us = bench("apply_batched", iters, || {
            let _ = apply_hashline_edits(&mid_content, &edits);
        });
        results.push(BenchResult {
            benchmark: "apply_batched".to_string(),
            file_lines: 1_000,
            edit_count: Some(num_edits),
            metric: "us_per_iter",
            value: (us * 10.0).round() / 10.0,
        });
    }

    results
}

fn print_markdown(results: &[BenchResult]) {
    println!("# Hashline Performance Benchmarks\n");

    println!("## format_hashlines\n");
    println!(
        "| {:>8} | {:>12} | {:>12} | {:>12} |",
        "Lines", "Time (us)", "Lines/sec", "MB/sec"
    );
    println!("|{:-<10}|{:-<14}|{:-<14}|{:-<14}|", "", "", "", "");
    let avg_line_bytes = 50.0_f64; // approximate
    for r in results.iter().filter(|r| r.benchmark == "format_hashlines") {
        let lines_per_sec = r.file_lines as f64 / (r.value / 1_000_000.0);
        let mb_per_sec =
            (r.file_lines as f64 * avg_line_bytes / 1_048_576.0) / (r.value / 1_000_000.0);
        println!(
            "| {:>8} | {:>12.1} | {:>12.0} | {:>12.1} |",
            r.file_lines, r.value, lines_per_sec, mb_per_sec
        );
    }

    println!("\n## compute_line_hash (per line)\n");
    println!(
        "| {:>8} | {:>12} | {:>12} |",
        "Lines", "Total (us)", "Per line (ns)"
    );
    println!("|{:-<10}|{:-<14}|{:-<14}|", "", "", "");
    for r in results
        .iter()
        .filter(|r| r.benchmark == "compute_line_hash")
    {
        let ns_per_line = r.value * 1000.0 / r.file_lines as f64;
        println!(
            "| {:>8} | {:>12.1} | {:>12.1} |",
            r.file_lines, r.value, ns_per_line
        );
    }

    println!("\n## parse_line_ref\n");
    println!(
        "| {:>12} | {:>12} | {:>12} |",
        "Iterations", "Total (us)", "Per call (ns)"
    );
    println!("|{:-<14}|{:-<14}|{:-<14}|", "", "", "");
    for r in results.iter().filter(|r| r.benchmark == "parse_line_ref") {
        let calls = r.edit_count.unwrap_or(1);
        let ns_per_call = r.value * 1000.0 / calls as f64;
        println!(
            "| {:>12} | {:>12.1} | {:>12.2} |",
            calls, r.value, ns_per_call
        );
    }

    println!("\n## apply_hashline_edits\n");
    println!("| {:>8} | {:>6} | {:>12} |", "Lines", "Edits", "Time (us)");
    println!("|{:-<10}|{:-<8}|{:-<14}|", "", "", "");
    for r in results
        .iter()
        .filter(|r| r.benchmark == "apply_hashline_edits")
    {
        println!(
            "| {:>8} | {:>6} | {:>12.1} |",
            r.file_lines,
            r.edit_count.unwrap_or(0),
            r.value
        );
    }

    println!("\n## apply_batched (1 000-line file)\n");
    println!(
        "| {:>12} | {:>12} | {:>16} |",
        "Edits batched", "Total (ms)", "Per edit (us)"
    );
    println!("|{:-<14}|{:-<14}|{:-<18}|", "", "", "");
    for r in results.iter().filter(|r| r.benchmark == "apply_batched") {
        let n = r.edit_count.unwrap_or(1) as f64;
        println!(
            "| {:>12} | {:>12.3} | {:>16.1} |",
            r.edit_count.unwrap_or(0),
            r.value / 1_000.0,
            r.value / n
        );
    }
}

fn main() {
    let json_mode = std::env::args().any(|a| a == "--json");

    let results = run_benchmarks();

    if json_mode {
        let report = BenchReport {
            version: env!("CARGO_PKG_VERSION").to_string(),
            commit: git_sha(),
            timestamp: iso_timestamp(),
            runner: std::env::var("BENCH_RUNNER").unwrap_or_else(|_| "local".to_string()),
            results,
        };
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        print_markdown(&results);
    }
}
