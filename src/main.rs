extern crate ctrlc;
extern crate env_logger;
extern crate failure;
#[macro_use]
extern crate log;
// Core
extern crate odin_core as odin;
// Backends
#[cfg(feature = "gstreamer")]
extern crate odin_gstreamer_backend as gst_backend;
// Frontends
#[cfg(feature = "web-api")]
extern crate odin_http_frontend as http_frontend;
// Provider
#[cfg(feature = "local-files")]
extern crate odin_local_provider as local_provider;
// Stores
extern crate odin_memory_store as memory_store;

#[cfg(feature = "sqlite-store")]
extern crate odin_sqlite_store as sqlite_store;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate structopt;
extern crate toml;

use std::path::Path;
use std::sync::{Arc, RwLock};

use failure::Error;
use log::LevelFilter;
use structopt::StructOpt;

use config::*;
#[cfg(feature = "gstreamer")]
use gst_backend::GstreamerPlayerBuilder;
use memory_store::MemoryLibrary;
use odin::extension::HostedExtension;
use odin::player::{queue::MemoryQueueBuilder, PlayerBuilder};
#[cfg(feature = "sqlite-store")]
use sqlite_store::SqliteLibrary;

mod config;
mod options;

fn main() -> Result<(), Error> {
    let options = options::CliOptions::from_args();
    let log_level = match options.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    env_logger::Builder::from_default_env()
        .filter(None, log_level)
        .init();

    let config = read_config(&options.config)?;

    let extensions = load_extensions(&options, &config)?;
    let providers = setup_providers(&config);

    let store: Box<dyn odin::Library> = match config.library.unwrap_or(LibraryConfig::Memory) {
        LibraryConfig::Memory => Box::new(MemoryLibrary::new()),
        #[cfg(feature = "sqlite-store")]
        LibraryConfig::SQLite { path } => Box::new(SqliteLibrary::new(path)?),
    };

    let app = odin::odin::new(store, providers, extensions)?;

    setup_interrupt(&app)?;

    for player_config in config.players.iter() {
        match player_config.backend_type {
            #[cfg(feature = "gstreamer")]
            PlayerBackend::GStreamer => {
                let player = PlayerBuilder::new(Arc::clone(&app))
                    .with_memory_queue()
                    .with_gstreamer()?
                    .build();
                app.add_player(player_config.name.clone(), player);
                if player_config.default {
                    app.set_default_player(player_config.name.clone());
                }
            }
        }
    }

    let mut threads = vec![
        odin::sync::start(Arc::clone(&app))?,
        odin::cache::start(Arc::clone(&app))?,
    ];



    #[cfg(feature = "web-api")]
    {
        if config.http.is_some() {
            let http_thread = http_frontend::start(config.http.clone(), Arc::clone(&app));
            threads.push(http_thread);
        }
    }





    for handle in threads {
        let _ = handle.join();
    }

    Ok(())
}

fn load_extensions(
    options: &options::CliOptions,
    config: &Config,
) -> Result<Vec<HostedExtension>, Error> {
    let mut paths = vec![
        Path::new("target/debug"),
        Path::new("target/release"),
        Path::new("extensions"),
    ];
    if let Some(ref path) = config.extensions.path {
        paths.insert(0, Path::new(path));
    }
    if let Some(ref path) = options.extensions_path {
        paths.insert(0, Path::new(path));
    }
    let path = paths.iter().find(|path| path.exists());
    if let Some(path) = path {
        let extensions = odin::extension::load_extensions(path)?;
        Ok(extensions)
    } else {
        Ok(Vec::new())
    }
}

fn setup_providers(config: &Config) -> odin::provider::SharedProviders {
    let mut providers: odin::provider::SharedProviders = vec![];

    #[cfg(feature = "local-files")]
    {
        if let Some(local) = config.local.clone() {
            providers.push(Arc::new(RwLock::new(Box::new(local))));
        }
    }
    for provider in &providers {
        let mut provider = provider.write().unwrap();
        provider
            .setup()
            .unwrap_or_else(|err| error!("Can't setup {} provider: {:?}", provider.title(), err));
    }

    providers
}

fn setup_interrupt(app: &Arc<odin::odin>) -> Result<(), Error> {
    let app = Arc::clone(app);
    ctrlc::set_handler(move || {
        app.exit();
    })?;
    Ok(())
}