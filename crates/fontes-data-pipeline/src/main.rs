use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "fontes-data",
    about = "Reproducible Fontes reading and study data compiler"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch locked source bytes, compile SQLite/FTS and package a complete profile.
    Build {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "study")]
        profile: String,
        #[arg(long, default_value = ".data-cache")]
        cache: PathBuf,
        #[arg(long, default_value = "dist/free-resources")]
        output: PathBuf,
        #[arg(long)]
        builder_commit: String,
        /// Refuse any network access; verified cache entries must already exist.
        #[arg(long)]
        offline: bool,
        /// Public repository whose prior releases retain source assets by digest.
        #[arg(long)]
        mirror_repository: Option<String>,
    },
    /// Recheck a compiled database against the permanent reference registry.
    Validate { database: PathBuf },
    /// Verify a signed release, decompress it with a size cap, and validate SQLite.
    Verify {
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        trust: PathBuf,
        #[arg(long)]
        repository: String,
    },
    /// Create a release key outside the repository; never prints its private seed.
    Keygen {
        #[arg(long)]
        secret_file: PathBuf,
        #[arg(long)]
        public_file: PathBuf,
        #[arg(long, default_value = "data-release-1")]
        key_id: String,
    },
    /// Sign and publish checked build outputs as a versioned GitHub Release.
    Publish {
        /// Legacy maintenance only: add a study build to a historical reading release bundle.
        #[arg(long)]
        additional_dir: Option<PathBuf>,
        #[arg(long)]
        dir: PathBuf,
        #[arg(long)]
        trust: PathBuf,
        #[arg(long)]
        repository: String,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Build {
            root,
            profile,
            cache,
            output,
            builder_commit,
            offline,
            mirror_repository,
        } => {
            fontes_data_pipeline::build(
                &root,
                &profile,
                &cache,
                &output,
                &builder_commit,
                offline,
                mirror_repository.as_deref(),
            )?;
        }
        Commands::Validate { database } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&fontes_data_pipeline::validate_database(&database)?)?
            );
        }
        Commands::Verify {
            dir,
            trust,
            repository,
        } => {
            fontes_data_pipeline::release::verify_package(&dir, &trust, &repository)?;
            println!("Release signature, artifacts, and SQLite verified");
        }
        Commands::Keygen {
            secret_file,
            public_file,
            key_id,
        } => {
            fontes_data_pipeline::release::keygen(&secret_file, &public_file, &key_id)?;
        }
        Commands::Publish {
            additional_dir,
            dir,
            trust,
            repository,
        } => {
            fontes_data_pipeline::release::publish(
                &dir,
                &trust,
                &repository,
                additional_dir.as_deref(),
            )?;
        }
    }
    Ok(())
}
