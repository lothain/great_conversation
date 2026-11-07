use std::{path::PathBuf, process::ExitCode};

use anyhow::Result;
use aph_data::{
    check::{self, Report, Severity},
    store::Dataset,
    wiki,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Validate and maintain the aphorisms dataset")]
struct Cli {
    /// Directory holding aphorisms.json, graph.json and civilizations.geojson.
    #[arg(long, default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data"))]
    data: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse every file into the typed model and run all semantic checks.
    Check {
        /// Also print warnings.
        #[arg(long)]
        warnings: bool,
    },
    /// Rewrite every file in canonical form (proves the model round-trips losslessly).
    Fmt,
    /// Match people and works to English Wikipedia articles.
    Wiki {
        /// Write the matches into the data files instead of only reporting them.
        #[arg(long)]
        apply: bool,
    },
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    let mut data = Dataset::load(&cli.data)?;

    match cli.command {
        Command::Check { warnings } => {
            let mut report = Report::default();
            check::collection(&data.collection, &mut report);
            check::graph(&data.graph, &mut report);
            check::cross(&data.collection, &data.graph, &mut report);
            check::civilizations(&data.civilizations, &mut report);
            report.0.sort_by_key(|p| p.severity);
            for p in report.0.iter().filter(|p| warnings || p.severity == Severity::Error) {
                println!("{p}");
            }
            let (errors, warns) = (report.count(Severity::Error), report.count(Severity::Warning));
            println!(
                "{} entries, {} authors, {} people, {} edges, {} civilization snapshots: {errors} errors, {warns} warnings",
                data.collection.entries.len(),
                data.collection.authors.len(),
                data.graph.people.len(),
                data.graph.edges.len(),
                data.civilizations.features.len(),
            );
            return Ok(if errors == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE });
        }
        Command::Fmt => data.save()?,
        Command::Wiki { apply } => {
            let mut client = wiki::Client::open(&cli.data.join("wiki_cache.json"))?;
            let matches = wiki::find(&mut client, &data.collection, &data.graph)?;
            let unmatched_people: Vec<_> = matches.people.iter().filter(|(_, t)| t.is_none()).map(|(id, _)| id.to_string()).collect();
            let unmatched_works: Vec<_> = matches.works.iter().filter(|(_, t)| t.is_none()).map(|((a, w), _)| format!("{a}: {w}")).collect();
            println!(
                "people: {}/{} matched; works: {}/{} matched",
                matches.people.len() - unmatched_people.len(),
                matches.people.len(),
                matches.works.len() - unmatched_works.len(),
                matches.works.len(),
            );
            for ((author, work), title) in &matches.works {
                match title {
                    Some(t) => println!("  work  {author}: {work}  =>  {t}"),
                    None => println!("  work  {author}: {work}  =>  (author article)"),
                }
            }
            for (id, title) in &matches.people {
                println!("  person {id}  =>  {}", title.as_deref().unwrap_or("-"));
            }
            if apply {
                wiki::apply(&matches, &mut data.collection, &mut data.graph);
                data.save()?;
                println!("written");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
