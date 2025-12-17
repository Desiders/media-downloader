use froodi::telers::setup_async_default;
use std::borrow::Cow;
use telers::{
    Bot, Dispatcher, Router,
    client::{
        Reqwest,
        telegram::{APIServer, BareFilesPathWrapper},
    },
    filters::Command,
};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _};

use crate::{
    config::{Config, get_config_path},
    presentation::tg_bot::{
        handlers::start,
        middlewares::CreateChatMiddleware,
        utils::{on_shutdown, on_startup},
    },
};

mod di_container;

pub mod adapters;
pub mod config;
pub mod entities;
pub mod errors;
pub mod interactors;
pub mod presentation;
pub mod utils;
pub mod value_objects;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), anyhow::Error> {
    let config_path = &*get_config_path();
    let config = Config::from_fs(config_path)?;

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::builder().parse_lossy(config.logging.dirs.as_ref()))
        .init();

    let base_url = format!("{}/bot{{token}}/{{method_name}}", config.tg_bot_api.url);
    let files_url = format!("{}/file{{token}}/{{path}}", config.tg_bot_api.url);

    let bot = Bot::with_client(
        config.bot.token.clone(),
        Reqwest::default().with_api_server(Cow::Owned(APIServer::new(&base_url, &files_url, true, BareFilesPathWrapper))),
    );

    let container = di_container::init(bot.clone(), config);

    let router = Router::new("main");
    let mut router = setup_async_default(router, container.clone());

    router.update.outer_middlewares.register(CreateChatMiddleware);
    router.message.register(start).filter(Command::many(["start", "help"]));

    let mut download_router = Router::new("download");

    router.include(download_router);

    router.startup.register(on_startup, (bot.clone(),));
    router.shutdown.register(on_shutdown, ());

    let dispatcher = Dispatcher::builder()
        .allowed_updates(router.resolve_used_update_types())
        .main_router(router.configure_default())
        .bot(bot)
        .build();

    match dispatcher.run_polling().await {
        Ok(()) => {
            info!("Bot stopped");
        }
        Err(err) => {
            error!(error = %err, "Bot stopped");
        }
    }

    container.close().await;

    Ok(())
}
