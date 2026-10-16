use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use fontes_resource::{InitOptions, Report, ValidationOptions};
use std::{fs, path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(
    name = "fontes-resource",
    version,
    about = "Scaffold, strictly validate, and package Fontes publisher source resources"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args)]
struct Checks {
    /// Local concept registry snapshot; checked-in taxonomy.example.xml is illustrative only.
    #[arg(long)]
    taxonomy: Option<PathBuf>,
    /// Fail on TODO placeholders and other authoring warnings as well as errors.
    #[arg(long)]
    deny_warnings: bool,
}
impl Checks {
    fn options(self) -> ValidationOptions {
        ValidationOptions {
            taxonomy: self.taxonomy,
            deny_warnings: self.deny_warnings,
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}
#[derive(Subcommand)]
enum Commands {
    /// Validate submissions, prepare release previews, and publish approved source packages.
    Submissions {
        #[command(subcommand)]
        command: SubmissionCommands,
    },
    /// Create a new source directory. No existing directory is overwritten.
    Init(Box<InitOptions>),
    /// Validate XML/XSD, cross-file references, assets, IDs, coverage, and concepts.
    Validate {
        source: PathBuf,
        #[command(flatten)]
        checks: Checks,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
        /// Also save the versioned JSON diagnostic report outside the source directory.
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Validate and create a deterministic source ZIP; never overwrite an existing ZIP.
    Pack {
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[command(flatten)]
        checks: Checks,
    },
    /// Export the exact XSDs embedded in this CLI to a new directory.
    Schemas {
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Args)]
struct SubmissionBuild {
    /// Library checkout containing submissions/.
    #[arg(long)]
    root: PathBuf,
    /// Trusted checkout containing policy.json, approvals/, taxonomy, and trust.
    #[arg(long)]
    policy_root: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    repository: String,
    #[arg(long)]
    source_repository: String,
    #[arg(long)]
    source_commit: String,
    #[arg(long)]
    tooling_commit: String,
}

#[derive(Subcommand)]
enum SubmissionCommands {
    /// Validate every submission and build unsigned preview assets; approvals are read from trusted policy.
    Check(SubmissionBuild),
    /// Prepare only versions with a trusted maintainer approval; no signing credentials needed.
    Build(SubmissionBuild),
    /// Sign, publish, and verify approved source releases using gh and FONTES_RESOURCE_SIGNING_KEY.
    Publish {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        policy_root: PathBuf,
        #[arg(long)]
        repository: String,
    },
    /// Authenticate a downloaded source release and every asset; does not install it.
    Verify {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        trust: PathBuf,
        #[arg(long)]
        repository: String,
    },
}

fn submissions(command: SubmissionCommands) -> Result<ExitCode> {
    use fontes_resource::publishing;
    let (options, approved_only) = match command {
        SubmissionCommands::Check(options) => (options, false),
        SubmissionCommands::Build(options) => (options, true),
        SubmissionCommands::Publish {
            dir,
            policy_root,
            repository,
        } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&publishing::publish(
                    &dir,
                    &policy_root,
                    &repository
                )?)?
            );
            return Ok(ExitCode::SUCCESS);
        }
        SubmissionCommands::Verify {
            dir,
            trust,
            repository,
        } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&publishing::verify(&dir, &trust, &repository)?)?
            );
            return Ok(ExitCode::SUCCESS);
        }
    };
    let result = publishing::build(&publishing::BuildOptions {
        root: &options.root,
        policy_root: &options.policy_root,
        output: &options.output,
        repository: &options.repository,
        source_repository: &options.source_repository,
        source_commit: &options.source_commit,
        tooling_commit: &options.tooling_commit,
        approved_only,
    })?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(if result.valid {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn show(report: &Report, format: Format) -> Result<()> {
    match format {
        Format::Text => print!("{}", report.text()),
        Format::Json => println!("{}", serde_json::to_string_pretty(report)?),
    }
    Ok(())
}
fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Commands::Submissions { command } => return submissions(command),
        Commands::Init(options) => {
            let report = fontes_resource::init(&options)?;
            println!("Created {}", options.directory.display());
            show(&report, Format::Text)?;
        }
        Commands::Validate {
            source,
            checks,
            format,
            report,
        } => {
            // A report inside the source would contaminate the next validation/pack.
            if let Some(path) = &report {
                let source_abs = fs::canonicalize(&source)?;
                let parent = path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(std::path::Path::new("."));
                let parent_abs = fs::canonicalize(parent)?;
                let target = parent_abs.join(
                    path.file_name()
                        .ok_or_else(|| anyhow::anyhow!("report requires a filename"))?,
                );
                anyhow::ensure!(
                    target != source_abs
                        && !(source_abs.is_dir() && target.starts_with(&source_abs)),
                    "write diagnostic reports outside the source directory/archive"
                );
                anyhow::ensure!(!path.exists(), "report path exists; choose a new path");
            }
            let result = fontes_resource::validate(&source, &checks.options())?;
            if let Some(path) = report {
                use std::io::Write;
                let mut file = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(path)?;
                file.write_all(serde_json::to_string_pretty(&result)?.as_bytes())?;
            }
            show(&result, format)?;
            if !result.valid {
                return Ok(ExitCode::from(1));
            }
        }
        Commands::Pack {
            source,
            output,
            checks,
        } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&fontes_resource::pack(
                    &source,
                    &output,
                    &checks.options()
                )?)?
            );
        }
        Commands::Schemas { output } => {
            fontes_resource::write_schemas(&output)?;
            println!("Wrote schemas to {}", output.display());
        }
    }
    Ok(ExitCode::SUCCESS)
}
fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}
