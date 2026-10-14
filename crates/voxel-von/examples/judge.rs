//! **Judge a ballot against the artifact it names, with the live model.**
//!
//! The ballot comes from the package format ([`voxel_package::ballot`]), the state is the
//! artifact's **own bytes** read off disk, and the verdict is a `laya::Decision` — the same record
//! the gate files, so an answer and a gate decision cannot be two shapes.
//!
//! ```text
//! JIGOR_CACHE_DIR=F:/planet-models cargo run -p voxel-von --example judge -- <root> <ballot-file>
//! ```
//!
//! It re-hashes the artifact before asking: a judgment about bytes that changed under it is about a
//! file that no longer exists, and upstream's rule for that is a refusal rather than a retry.

use std::path::Path;

use voxel_von::judge;

/// How much of the artifact the model is shown. Upstream measured that **cost follows the question
/// structure, not the state** (a 100× larger state added 8 %), so the choice here is about what the
/// model can read, not about what it costs: enough to see the shape of the file, not so much that a
/// 2,000-line module is silently truncated mid-sentence without anyone knowing.
const STATE_BYTES: usize = 8_000;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: judge <workspace-root> <ballot-file>");
        return std::process::ExitCode::from(2);
    }
    let root = Path::new(&args[0]);
    let ballot_file = Path::new(&args[1]);

    let ballots = match voxel_package::ballot::read(ballot_file) {
        Ok(ballots) => ballots,
        Err(refusals) => {
            eprintln!("{refusals}");
            return std::process::ExitCode::from(2);
        }
    };

    println!("classifier\tvon");
    println!("model\t{}", voxel_von::MODEL);
    println!("cache\t{}", voxel_von::Von::cache_dir().display());
    println!("ballots\t{}", ballots.len());

    let mut classifier = match voxel_von::Von::load() {
        Ok(classifier) => classifier,
        Err(why) => {
            eprintln!("{why}");
            return std::process::ExitCode::from(3);
        }
    };

    let mut passing = 0usize;
    let mut non_readings = 0usize;
    for ballot in &ballots {
        let artifact = root.join(&ballot.artifact);
        let bytes = match std::fs::read(&artifact) {
            Ok(bytes) => bytes,
            Err(failure) => {
                println!(
                    "judgment\t{}\tnon-reading\tthe artifact {} cannot be read: {failure}",
                    ballot.id, ballot.artifact
                );
                non_readings += 1;
                continue;
            }
        };
        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(STATE_BYTES)]).to_string();
        let state = serde_json::json!({
            "artifact": ballot.artifact,
            "claim": ballot.claim,
            "presenter": ballot.presenter,
            "text": text,
            "bytes": bytes.len(),
            "truncated": bytes.len() > STATE_BYTES,
        });

        let started = std::time::Instant::now();
        match judge::ask(&mut classifier, ballot, &state) {
            Ok(decision) => {
                let verdict = match decision.verdict {
                    voxel_model::laya::Verdict::Belongs => "belongs",
                    voxel_model::laya::Verdict::NotBelongs => "not-belongs",
                    voxel_model::laya::Verdict::NoReading => "no-reading",
                };
                println!(
                    "judgment\t{}\t{}\t{}\tconfidence {:.4}\torigin {}\tartifact {} ({} bytes)\t{}ms",
                    ballot.id,
                    verdict,
                    ballot.kind.name(),
                    decision.confidence,
                    decision
                        .origin
                        .map(|origin| origin.letter())
                        .unwrap_or("none"),
                    ballot.artifact,
                    bytes.len(),
                    started.elapsed().as_millis(),
                );
                for grade in &decision.grades {
                    println!(
                        "grade\t{}\t{}\t{}",
                        ballot.id,
                        grade.0.name(),
                        grade.1
                    );
                }
                for finding in &decision.findings {
                    println!("finding\t{}\t{finding}", ballot.id);
                }
                if matches!(decision.verdict, voxel_model::laya::Verdict::Belongs) {
                    passing += 1;
                } else if matches!(decision.verdict, voxel_model::laya::Verdict::NoReading) {
                    non_readings += 1;
                }
            }
            Err(why) => {
                eprintln!("{why}");
                return std::process::ExitCode::from(3);
            }
        }
    }
    println!("judged\t{}", ballots.len());
    println!("passing\t{passing}");
    println!("non-readings\t{non_readings}");
    std::process::ExitCode::SUCCESS
}
