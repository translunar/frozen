use std::path::PathBuf;

use clap::{Parser, Subcommand};
use elfo_catalog::seedcache::SeedCache;
use elfo_catalog::GenOptions;

#[derive(Parser)]
#[command(name = "elfo-catalog")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate the frozen-orbit family catalog from a TOML config.
    Gen {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Directory of committed first-member seeds. Defaults to the repo's
        /// `seeds/`, or `$ELFO_SEEDS_DIR` if set.
        #[arg(long)]
        seeds: Option<PathBuf>,
        /// Update the seed cache from this run: write every converged first member
        /// and every confirmed absence back into the seeds directory.
        #[arg(long)]
        write_seeds: bool,
        /// Attempt families listed in the seeds directory's `absent.json` anyway.
        /// Also settable as `ELFO_RETRY_ABSENT=1`.
        #[arg(long)]
        retry_absent: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Gen { config, out, seeds, write_seeds, retry_absent } => {
            let defaults = GenOptions::default();
            let opts = GenOptions {
                seeds: seeds.map(SeedCache::new).unwrap_or(defaults.seeds),
                write_seeds,
                // The flag is an override in one direction only: `--retry-absent`
                // turns retrying on, and its absence leaves the env var in charge.
                retry_absent: retry_absent || defaults.retry_absent,
            };
            elfo_catalog::run_with(&config, &out, &opts)
        }
    }
}
