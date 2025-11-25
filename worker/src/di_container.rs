use froodi::{DefaultScope::App, async_impl::Container, async_registry, instance, registry};

use crate::config::{Config, Version};

pub fn init(config: Config, version: Version) -> Container {
    let sync_registry = registry! {
        scope(App) [
            provide(instance(config.logging)),
            provide(instance(config.limits)),
            provide(instance(config.yt_dlp)),
            provide(instance(config.yt_pot_provider)),
            provide(instance(version)),
        ]
    };
    let registry = async_registry! {
        extend(sync_registry)
    };

    Container::new(registry)
}
