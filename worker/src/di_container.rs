use froodi::{DefaultScope::App, Inject, async_impl::Container, async_registry, instance, registry};

use crate::{
    config::{Config, Limits, Version, YtDlp, YtPotProvider},
    interactors::download::{audio, thumbnail, video},
};

pub fn init(config: Config, version: Version) -> Container {
    let sync_registry = registry! {
        scope(App) [
            provide(instance(config.logging)),
            provide(instance(config.limits)),
            provide(instance(config.yt_dlp)),
            provide(instance(config.yt_pot_provider)),
            provide(instance(version)),

            provide(|| Ok(thumbnail::Download)),
            provide(|
                Inject(yt_dlp): Inject<YtDlp>,
                Inject(limits): Inject<Limits>,
                Inject(yt_pot): Inject<YtPotProvider>,| Ok(video::Download::new(yt_dlp, limits, yt_pot))),
            provide(|
                Inject(yt_dlp): Inject<YtDlp>,
                Inject(limits): Inject<Limits>,
                Inject(yt_pot): Inject<YtPotProvider>,| Ok(audio::Download::new(yt_dlp, limits, yt_pot))),
        ],
    };
    let registry = async_registry! {
        extend(sync_registry)
    };

    Container::new(registry)
}
