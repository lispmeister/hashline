use hashline::edit::{InsertAfterOp, SetLineOp};
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
    let _ = name; // used by caller
    per_iter_us
}

fn main() {
    let sizes = [100, 1_000, 10_000];

    println!("# Hashline Performance Benchmarks\n");

    // --- format_hashlines ---
    println!("## format_hashlines\n");
    println!(
        "| {:>8} | {:>12} | {:>12} | {:>12} |",
        "Lines", "Time (us)", "Lines/sec", "MB/sec"
    );
    println!("|{:-<10}|{:-<14}|{:-<14}|{:-<14}|", "", "", "", "");

    for &size in &sizes {
        let content = generate_file(size);
        let bytes = content.len();
        let us = bench("format_hashlines", 50, || {
            let _ = format_hashlines(&content, 1);
        });
        let lines_per_sec = size as f64 / (us / 1_000_000.0);
        let mb_per_sec = (bytes as f64 / 1_048_576.0) / (us / 1_000_000.0);
        println!(
            "| {:>8} | {:>12.1} | {:>12.0} | {:>12.1} |",
            size, us, lines_per_sec, mb_per_sec
        );
    }

    // --- compute_line_hash ---
    println!("\n## compute_line_hash (per line)\n");
    println!(
        "| {:>8} | {:>12} | {:>12} |",
        "Lines", "Total (us)", "Per line (ns)"
    );
    println!("|{:-<10}|{:-<14}|{:-<14}|", "", "", "");

    for &size in &sizes {
        let content = generate_file(size);
        let lines: Vec<&str> = content.split('\n').collect();
        let us = bench("compute_line_hash", 50, || {
            for (i, line) in lines.iter().enumerate() {
                let _ = compute_line_hash(i + 1, line);
            }
        });
        let ns_per_line = us * 1000.0 / size as f64;
        println!("| {:>8} | {:>12.1} | {:>12.1} |", size, us, ns_per_line);
    }

    // --- parse_line_ref ---
    println!("\n## parse_line_ref\n");
    println!(
        "| {:>12} | {:>12} | {:>12} |",
        "Iterations", "Total (us)", "Per call (ns)"
    );
    println!("|{:-<14}|{:-<14}|{:-<14}|", "", "", "");

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
    let ns_per_call = us * 1000.0 / calls as f64;
    println!("| {:>12} | {:>12.1} | {:>12.2} |", calls, us, ns_per_call);

    // --- apply_hashline_edits ---
    println!("\n## apply_hashline_edits\n");
    println!("| {:>8} | {:>6} | {:>12} |", "Lines", "Edits", "Time (us)");
    println!("|{:-<10}|{:-<8}|{:-<14}|", "", "", "");

    for &size in &sizes {
        let content = generate_file(size);
        let lines: Vec<&str> = content.split('\n').collect();

        for &num_edits in &[1, 5, 20] {
            if num_edits > size {
                continue;
            }
            // Create edits spread across the file
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

            let us = bench("apply_edits", 50, || {
                let _ = apply_hashline_edits(&content, &edits);
            });
            println!("| {:>8} | {:>6} | {:>12.1} |", size, num_edits, us);
        }
    }

    // --- apply with insert_after ---
    println!("\n## apply_hashline_edits (insert_after)\n");
    println!(
        "| {:>8} | {:>6} | {:>12} |",
        "Lines", "Inserts", "Time (us)"
    );
    println!("|{:-<10}|{:-<8}|{:-<14}|", "", "", "");

    for &size in &sizes {
        let content = generate_file(size);
        let lines: Vec<&str> = content.split('\n').collect();

        for &num_edits in &[1, 5, 20] {
            if num_edits > size {
                continue;
            }
            let edits: Vec<HashlineEdit> = (0..num_edits)
                .map(|i| {
                    let line_idx = (i * size / num_edits).min(size - 1);
                    let line_num = line_idx + 1;
                    let hash = compute_line_hash(line_num, lines[line_idx]);
                    HashlineEdit::InsertAfter {
                        insert_after: InsertAfterOp {
                            anchor: format!("{}:{}", line_num, hash),
                            text: Some("    // inserted line".to_string()),
                            content: None,
                        },
                    }
                })
                .collect();

            let us = bench("apply_inserts", 50, || {
                let _ = apply_hashline_edits(&content, &edits);
            });
            println!("| {:>8} | {:>6} | {:>12.1} |", size, num_edits, us);
        }
    }
}
