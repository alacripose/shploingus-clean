//! What the materials tree can offer a given inventory — the lookup and craft act, as a command.
//!
//! ```text
//! USAGE: materials-options [--stock NAME=GRAMS]... [--word KIND=NAME]... [--craft PROCESS] [--all]
//!
//!   --stock   grams held of a declared substance; a fraction is `num/den`
//!   --word    a structure, tool or skill the player has
//!   --craft   perform one row against the stock after listing the offers
//!   --all     list every blocked row and what it is short of, not only the ready ones
//!
//! Exit 0 — the stock was read and every offer printed.
//! Exit 1 — a row or a stock amount was refused.
//! Exit 2 — a usage error.
//! ```
//!
//! The library is the surface ([`voxel_model::materials::stock`]); this binary exists so the same
//! path is exercised by a person, not only by a test. It never parses a declaration file: the table
//! is compiled in.

use std::process::ExitCode;

use voxel_model::material::Ratio;
use voxel_model::materials::generated;
use voxel_model::materials::schema::VocabKind;
use voxel_model::materials::stock::Stock;

const HELP: &str = "\
what the materials tree can offer a given inventory\n\n\
USAGE: materials-options [--stock NAME=GRAMS]... [--word KIND=NAME]... [--craft PROCESS] [--all]\n\n\
  --stock   grams held of a declared substance; a fraction is `num/den`\n\
  --word    a structure, tool or skill the player has\n\
  --craft   perform one row against the stock after listing the offers\n\
  --all     list every blocked row and what it is short of, not only the ready ones";

fn main() -> ExitCode {
    let mut stock = Stock::new();
    let mut craft: Option<String> = None;
    let mut all = false;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--help" | "-h" => {
                println!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "--all" => all = true,
            "--stock" => {
                let Some(value) = arguments.next() else {
                    eprintln!("materials-options: --stock needs NAME=GRAMS");
                    return ExitCode::from(2);
                };
                match parse_stock(&value) {
                    Ok((name, grams)) => stock.hold(name, grams),
                    Err(why) => {
                        eprintln!("materials-options: {why}");
                        return ExitCode::from(2);
                    }
                }
            }
            "--word" => {
                let Some(value) = arguments.next() else {
                    eprintln!("materials-options: --word needs KIND=NAME");
                    return ExitCode::from(2);
                };
                match parse_word(&value) {
                    Ok((kind, name)) => stock.learn(kind, name),
                    Err(why) => {
                        eprintln!("materials-options: {why}");
                        return ExitCode::from(2);
                    }
                }
            }
            "--craft" => match arguments.next() {
                Some(value) if !value.trim().is_empty() => craft = Some(value),
                _ => {
                    eprintln!("materials-options: --craft needs a process name");
                    return ExitCode::from(2);
                }
            },
            other => {
                eprintln!("materials-options: `{other}` is not an argument this command takes; try --help");
                return ExitCode::from(2);
            }
        }
    }

    let table = generated::table();
    let coverage = table.tree().coverage();
    println!(
        "materials: {} tiers, {} substances, {} processes",
        coverage.tiers, coverage.substances, coverage.processes
    );
    if stock.ledger().is_empty() {
        println!("stock: empty");
    } else {
        let held: Vec<String> = stock
            .ledger()
            .iter()
            .map(|(name, grams)| format!("{name} {grams} g"))
            .collect();
        println!("stock: {}", held.join(", "));
    }

    let offers = match table.offers(&stock) {
        Ok(offers) => offers,
        Err(why) => {
            eprintln!("materials-options: the tree will not offer: {why}");
            return ExitCode::FAILURE;
        }
    };
    let ready = offers.iter().filter(|offer| offer.is_ready()).count();
    println!("options: {} (ready {ready})", offers.len());
    for offer in &offers {
        if offer.is_ready() {
            let outputs: Vec<String> = offer
                .process
                .outputs
                .iter()
                .map(|qty| format!("{} {}", qty.amount, qty.substance))
                .collect();
            println!("  ready    {}  {} -> {}", offer.tier.name, offer.process.name, outputs.join(", "));
        } else if all {
            println!(
                "  blocked  {}  {}  [{}]",
                offer.tier.name,
                offer.process.name,
                offer.shortfall()
            );
        }
    }
    if !all && ready < offers.len() {
        println!("blocked: {} (use --all)", offers.len() - ready);
    }

    if let Some(name) = craft {
        match table.craft(&name, &mut stock) {
            Ok(outputs) => {
                println!("crafted: {name}");
                for qty in outputs {
                    println!("  made  {} {}", qty.amount, qty.substance);
                }
                let left: Vec<String> = stock
                    .ledger()
                    .iter()
                    .map(|(substance, grams)| format!("{substance} {grams} g"))
                    .collect();
                println!("stock: {}", if left.is_empty() { "empty".into() } else { left.join(", ") });
            }
            Err(why) => {
                eprintln!("materials-options: {why}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

/// `NAME=GRAMS`, where grams is a whole number or `num/den`.
fn parse_stock(value: &str) -> Result<(String, Ratio), String> {
    let (name, amount) = value
        .split_once('=')
        .ok_or_else(|| format!("`{value}` is not NAME=GRAMS"))?;
    if name.trim().is_empty() {
        return Err(format!("`{value}` has no substance name"));
    }
    let grams = match amount.split_once('/') {
        Some((num, den)) => {
            let num: u64 = num.parse().map_err(|_| format!("`{amount}` is not num/den"))?;
            let den: u64 = den.parse().map_err(|_| format!("`{amount}` is not num/den"))?;
            Ratio::new(num, den).map_err(|why| why.detail().to_string())?
        }
        None => {
            let grams: u64 = amount.parse().map_err(|_| format!("`{amount}` is not a mass in grams"))?;
            Ratio::from_u64(grams)
        }
    };
    Ok((name.to_string(), grams))
}

/// `KIND=NAME`, where kind is `structure`, `tool` or `skill`.
fn parse_word(value: &str) -> Result<(VocabKind, String), String> {
    let (kind, name) = value
        .split_once('=')
        .ok_or_else(|| format!("`{value}` is not KIND=NAME"))?;
    let kind = match kind {
        "structure" => VocabKind::Structure,
        "tool" => VocabKind::Tool,
        "skill" => VocabKind::Skill,
        other => return Err(format!("`{other}` is not structure, tool or skill")),
    };
    if name.trim().is_empty() {
        return Err(format!("`{value}` has no name"));
    }
    Ok((kind, name.to_string()))
}
