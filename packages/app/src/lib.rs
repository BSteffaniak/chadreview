#![cfg_attr(feature = "fail-on-warnings", deny(warnings))]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

pub mod actions;
pub mod events;
pub mod routes;

#[cfg(feature = "assets")]
pub mod assets {
    use std::{path::PathBuf, sync::LazyLock};

    use hyperchad::renderer;

    static CARGO_MANIFEST_DIR: LazyLock<Option<std::path::PathBuf>> =
        LazyLock::new(|| std::option_env!("CARGO_MANIFEST_DIR").map(Into::into));

    static ASSETS_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
        CARGO_MANIFEST_DIR.as_ref().map_or_else(
            || <PathBuf as std::str::FromStr>::from_str("public").unwrap(),
            |dir| dir.join("public"),
        )
    });

    pub static ASSETS: LazyLock<Vec<renderer::assets::StaticAssetRoute>> = LazyLock::new(|| {
        vec![
            #[cfg(feature = "vanilla-js")]
            renderer::assets::StaticAssetRoute {
                route: format!(
                    "/js/{}",
                    hyperchad::renderer_vanilla_js::SCRIPT_NAME_HASHED.as_str()
                ),
                target: renderer::assets::AssetPathTarget::FileContents(
                    hyperchad::renderer_vanilla_js::SCRIPT.as_bytes().into(),
                ),
            },
            renderer::assets::StaticAssetRoute {
                route: "/public".to_string(),
                target: ASSETS_DIR.clone().try_into().unwrap(),
            },
        ]
    });
}
