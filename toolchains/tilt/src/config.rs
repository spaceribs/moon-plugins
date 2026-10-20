use moon_pdk_api::{UnresolvedVersionSpec, config_struct};

config_struct!(
    /// Configures and enables the Tilt toolchain in `.moon/toolchains.yml`.
    ///
    /// ```yaml
    /// tilt:
    ///   plugin: "github://spaceribs/moon-plugins/tilt_toolchain"
    ///   version: "0.37.8"
    /// ```
    #[derive(Default, schematic::Schematic)]
    pub struct TiltToolchainConfig {
        /// Version of Tilt to download, install, and run tasks with.
        pub version: Option<UnresolvedVersionSpec>,
    }
);
