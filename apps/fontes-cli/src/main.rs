use clap::{Parser, Subcommand};
use fontes_engine::{Command, Engine, command_catalog, transport};
use std::{io, path::PathBuf};
#[derive(Parser)]
#[command(about = "Headless Fontes application commands, JSON control and MCP stdio")]
struct Args {
    #[arg(long)]
    library: Option<PathBuf>,
    #[arg(long)]
    user_db: Option<PathBuf>,
    #[command(subcommand)]
    command: Mode,
}
#[derive(Subcommand)]
enum Mode {
    /// Download and verify a first-run library without needing a database path.
    Setup {
        #[arg(long)]
        data_dir: PathBuf,
        #[arg(long, default_value = "study", value_parser = ["reading", "study"])]
        profile: String,
    },
    /// List stable command IDs and generated input schemas; no database needed.
    Commands,
    /// Dispatch a stable command ID with a JSON object of parameters.
    Run {
        id: String,
        #[arg(long, default_value = "{}")]
        params: String,
    },
    /// Read one command envelope per line, return one result/error per line.
    Json,
    /// Serve Model Context Protocol tools over stdio; keep stdout protocol-only.
    Mcp,
}
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if matches!(args.command, Mode::Commands) {
        println!("{}", serde_json::to_string_pretty(&command_catalog())?);
        return Ok(());
    }
    if let Mode::Setup { data_dir, profile } = &args.command {
        let profile = if profile == "study" {
            fontes_engine::bootstrap::Profile::Study
        } else {
            fontes_engine::bootstrap::Profile::Reading
        };
        let path = fontes_engine::bootstrap::install(
            data_dir,
            profile,
            &std::sync::atomic::AtomicBool::new(false),
            |p| {
                if p.total.is_none() {
                    eprintln!("{}", p.message);
                }
            },
        )?;
        println!(
            "{}",
            serde_json::json!({"operation": fontes_engine::bootstrap::SETUP_OPERATION, "library": path, "verified": true, "profile": profile.id()})
        );
        return Ok(());
    }
    let mut engine = Engine::open(
        &args
            .library
            .ok_or_else(|| anyhow::anyhow!("--library is required"))?,
        &args.user_db.ok_or_else(|| {
            anyhow::anyhow!("--user-db is required; use the app's user.sqlite to share notes")
        })?,
    )?;
    match args.command {
        Mode::Run { id, params } => {
            let args: serde_json::Value = serde_json::from_str(&params)?;
            let command: Command =
                serde_json::from_value(serde_json::json!({"command":id,"params":args}))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&engine.dispatch(command)?)?
            );
        }
        Mode::Json | Mode::Mcp => transport::serve(
            &mut engine,
            io::stdin().lock(),
            io::stdout().lock(),
            matches!(args.command, Mode::Mcp),
        )?,
        Mode::Commands | Mode::Setup { .. } => unreachable!(),
    }
    Ok(())
}
