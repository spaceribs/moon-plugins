use crate::config::TiltToolchainConfig;
use extism_pdk::*;
use moon_pdk_api::*;

#[plugin_fn]
pub fn register_toolchain(
    Json(_): Json<RegisterToolchainInput>,
) -> FnResult<Json<RegisterToolchainOutput>> {
    Ok(Json(RegisterToolchainOutput {
        name: "Tilt".into(),
        description: Some("Installs and pins Tilt.".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        // Tasks that run `tilt` get the pinned version on PATH. Projects are
        // not marked Tilt-aware by their Tiltfile, otherwise every task in them
        // would require installing Tilt.
        exe_names: vec!["tilt".into()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_toolchain_config() -> FnResult<Json<DefineToolchainConfigOutput>> {
    Ok(Json(DefineToolchainConfigOutput {
        schema: schematic::SchemaBuilder::build_root::<TiltToolchainConfig>(),
    }))
}

#[plugin_fn]
pub fn initialize_toolchain(
    Json(_): Json<InitializeToolchainInput>,
) -> FnResult<Json<InitializeToolchainOutput>> {
    Ok(Json(InitializeToolchainOutput {
        config_url: Some(
            "https://github.com/spaceribs/moon-plugins/tree/main/toolchains/tilt".into(),
        ),
        docs_url: Some("https://docs.tilt.dev".into()),
        ..Default::default()
    }))
}
