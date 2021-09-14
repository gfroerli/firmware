//! Flash the EEPROM config based on a toml configfile.

use std::{fs, path::PathBuf, process::exit};

use anyhow::{Context, Result};
use clap::Clap;
use gfroerli_common::config::Config;
use probe_rs::{
    config::{MemoryRegion, NvmRegion},
    flashing::{BinOptions, FlashLoader, FlashProgress, ProgressEvent},
    Probe,
};

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Clap)]
struct Opts {
    /// Path to a configuration file in TOML format.
    #[clap(short, long)]
    config: PathBuf,
}

fn main() -> Result<()> {
    // Parse command line args
    let opts: Opts = Opts::parse();

    // Parse config
    let config_source = fs::read_to_string(&opts.config).context("Could not read config file")?;
    let config: Config = toml::from_str(&config_source).context("Could not parse config file")?;
    let data = config.serialize();

    // Get a list of all available debug probes
    let probes = Probe::list_all();
    if probes.is_empty() {
        println!("No probes found");
        exit(1);
    }
    println!("Probes found:");
    for probe in &probes {
        println!("- {:?}", probe);
    }
    println!("Using first entry");

    // Use the first probe found
    let probe = probes[0].open()?;

    // Attach to a chip
    println!("Attaching chip");
    let mut session = probe.attach("STM32L071KBTx")?;

    // Memory map: We only include the EEPROM memory region. This way, we can
    // also prevent accidentally writing to flash.
    let memory_map = vec![MemoryRegion::Nvm(NvmRegion {
        range: (0x0808_0000..0x0808_0200),
        is_boot_memory: false,
    })];

    // Initialize flash loader
    println!("Initialize flash loader");
    let keep_unwritten_bytes = false;
    let mut loader = FlashLoader::new(
        memory_map,
        keep_unwritten_bytes,
        session.target().source().clone(),
    );

    // Load data
    println!("Load data");
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&data);
    loader
        .load_bin_data(
            &mut buffer,
            &mut cursor,
            BinOptions {
                base_address: Some(0x0808_0000),
                skip: 0,
            },
        )
        .context("Call to load_bin_data failed")?;

    // Progress function
    let progress = FlashProgress::new(|ev: ProgressEvent| {
        println!("Progress: {:?}", ev);
    });

    // Write data
    println!("Commit write");
    let do_chip_erase = false;
    let dry_run = false;
    loader
        .commit(&mut session, &progress, do_chip_erase, dry_run)
        .context("Could not commit flash loader")?;

    // Reset
    println!("Attaching to core in order to reset");
    let mut core = session.core(0).context("Failed to attach to core")?;
    core.reset().context("Failed to reset core")?;

    Ok(())
}
