use froodi::{DefaultScope::App, async_impl::Container, async_registry, instance, registry};

use crate::config::Config;

pub fn init(config: Config) -> Container {
    let sync_registry = registry! {
        scope(App) [
            provide(instance(config.logging)),
            provide(instance(config.limits)),
        ],
    };
    let registry = async_registry! {
        extend(sync_registry)
    };

    Container::new(registry)
}
