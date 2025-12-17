use froodi::{
    DefaultScope::{App, Request},
    Inject, InstantiateErrorKind,
    async_impl::Container,
    async_registry, instance, registry,
};
use reqwest::Client;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::sync::{Arc, Mutex};
use telers::Bot;
use tracing::{error, info};
use uuid::ContextV7;

use crate::{adapters::database::TxManager, config, config::Config, interactors::SaveChat};

pub fn init(bot: Bot, config: Config) -> Container {
    let sync_registry = registry! {
        scope(App) [
            provide(instance(bot)),
            provide(instance(config.bot)),
            provide(instance(config.chat)),
            provide(instance(config.blacklisted)),
            provide(instance(config.logging)),
            provide(instance(config.database)),
            provide(instance(config.limits)),
            provide(instance(config.tg_bot_api)),

            provide(|| Ok(Mutex::new(ContextV7::new()))),
            provide(|| Ok(Client::new())),
            provide(|| Ok(SaveChat::new())),
        ],
    };
    let registry = async_registry! {
        provide(
            App,
            |Inject(database_cfg): Inject<config::Database>| async move {
                let mut options = ConnectOptions::new(database_cfg.get_postgres_url());
                options.sqlx_logging(false);

                match Database::connect(options).await {
                    Ok(database_conn) => {
                        info!("Database conn created");
                        Ok(database_conn)
                    }
                    Err(err) => {
                        error!(%err, "Create database conn err");
                        Err(InstantiateErrorKind::Custom(err.into()))
                    }
                }
            },
            finalizer = |database_conn: Arc<DatabaseConnection>| async move {
                match database_conn.close_by_ref().await {
                    Ok(()) => {
                        info!("Database conn closed");
                    },
                    Err(err) => {
                        error!(%err, "Close database conn err");
                    },
                }
            },
         ),
        provide(
            Request,
            |Inject(pool): Inject<DatabaseConnection>| async move { Ok(TxManager::new(pool)) },
        ),
        extend(sync_registry),
    };

    Container::new(registry)
}
