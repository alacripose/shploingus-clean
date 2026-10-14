//! **Ask the classifier, the way a judge would**: one request in, the answers out.
//!
//! The request is JSON on stdin — a state and a list of questions in the question shapes the
//! decision layer declares:
//!
//! ```json
//! {"state": {"artifact": "...", "text": "..."},
//!  "questions": [
//!    {"id": "belongs", "kind": "noul", "instructions": "does this belong to the recipe?"},
//!    {"id": "rung", "kind": "score", "instructions": "how far does it get?",
//!     "levels": ["not at all", "partly", "mostly", "wholly"]}
//!  ]}
//! ```
//!
//! Run it with the model cache named, and nothing else:
//!
//! ```text
//! JIGOR_CACHE_DIR=F:/planet-models cargo run -p voxel-von --example ask < request.json
//! ```
//!
//! It prints one `key<TAB>value` line per answer, plus the model id and the cache it read — the
//! same discipline the gate's reports use, so an answer and the report about it cannot disagree.

use std::io::Read;

use jigor::Question;
use serde_json::{json, Value};
use voxel_von::{to_wire, Von};

fn question_of(value: &Value) -> Result<Question, String> {
    let id = value.get("id").and_then(Value::as_str).unwrap_or("q");
    let kind = value.get("kind").and_then(Value::as_str).unwrap_or("noul");
    let instructions = value
        .get("instructions")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{id} has no instructions; a question with none asks nothing"))?;
    let words = |field: &str| -> Vec<String> {
        value
            .get(field)
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };
    match kind {
        "noul" => Ok(voxel_von::noul(id, instructions)),
        "choice" => {
            let options = words("options");
            if options.is_empty() {
                return Err(format!(
                    "{id} is a choice with no options; a choice with one option is not a choice"
                ));
            }
            let borrowed: Vec<&str> = options.iter().map(String::as_str).collect();
            Ok(voxel_von::choice(id, instructions, &borrowed))
        }
        "score" => {
            let levels = words("levels");
            if levels.len() < 2 {
                return Err(format!(
                    "{id} is a score over {} level(s); a scale needs at least two",
                    levels.len()
                ));
            }
            let borrowed: Vec<&str> = levels.iter().map(String::as_str).collect();
            Ok(voxel_von::score(id, instructions, &borrowed))
        }
        other => Err(format!(
            "{other} is not a question kind; the kinds are noul, choice and score"
        )),
    }
}

fn main() -> std::process::ExitCode {
    let mut request = String::new();
    if let Err(failure) = std::io::stdin().read_to_string(&mut request) {
        eprintln!("the request could not be read: {failure}");
        return std::process::ExitCode::from(2);
    }
    let request: Value = match serde_json::from_str(&request) {
        Ok(value) => value,
        Err(failure) => {
            eprintln!("the request is not JSON: {failure}");
            return std::process::ExitCode::from(2);
        }
    };

    let built: Result<Vec<Question>, String> = request
        .get("questions")
        .and_then(Value::as_array)
        .ok_or_else(|| "the request carries no `questions` array".to_string())
        .and_then(|list| list.iter().map(question_of).collect());
    let questions = match built {
        Ok(questions) if !questions.is_empty() => questions,
        Ok(_) => {
            eprintln!("the request carries no questions");
            return std::process::ExitCode::from(2);
        }
        Err(why) => {
            eprintln!("{why}");
            return std::process::ExitCode::from(2);
        }
    };

    println!("classifier\tvon");
    println!("model\t{}", voxel_von::MODEL);
    println!("cache\t{}", Von::cache_dir().display());
    println!("cached\t{}", voxel_von::cached());
    println!("questions\t{}", questions.len());

    let started = std::time::Instant::now();
    let mut classifier = match Von::load() {
        Ok(classifier) => classifier,
        Err(why) => {
            eprintln!("{why}");
            return std::process::ExitCode::from(3);
        }
    };
    let load = started.elapsed();

    let state = request.get("state").cloned().unwrap_or_else(|| json!({}));
    let asked = std::time::Instant::now();
    let answers = match classifier.answers(&state, &questions) {
        Ok(answers) => answers,
        Err(why) => {
            eprintln!("{why}");
            return std::process::ExitCode::from(3);
        }
    };
    let ask = asked.elapsed();

    // In the order asked, never in map order: a report whose rows move is a report nobody can diff.
    for question in &questions {
        match answers.get(&question.id) {
            Some(answer) => println!(
                "answer\t{}\t{}\t{}",
                question.id,
                answer.kind(),
                to_wire(answer)
            ),
            None => println!("answer\t{}\tmissing\t", question.id),
        }
    }
    println!("load-ms\t{}", load.as_millis());
    println!("ask-ms\t{}", ask.as_millis());
    std::process::ExitCode::SUCCESS
}
