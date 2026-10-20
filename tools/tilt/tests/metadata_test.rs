use proto_pdk_test_utils::*;

mod tilt_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("tilt-test").await;

        let metadata = plugin
            .register_tool(RegisterToolInput {
                id: Id::raw("tilt"),
            })
            .await;

        assert_eq!(metadata.name, "Tilt");
        assert_eq!(metadata.type_of, PluginType::CommandLine);
        assert_eq!(
            metadata.plugin_version.unwrap().to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_tilt_executable() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("tilt-test", |config| {
                config.host(HostOS::Linux, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput::default())
            .await;

        let exe = output.exes.get("tilt").unwrap();

        assert!(exe.primary);
        assert_eq!(exe.exe_path, Some("tilt".into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_tilt_executable_on_windows() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_plugin_with_config("tilt-test", |config| {
                config.host(HostOS::Windows, HostArch::X64);
            })
            .await;

        let output = plugin
            .locate_executables(LocateExecutablesInput::default())
            .await;

        assert_eq!(
            output.exes.get("tilt").unwrap().exe_path,
            Some("tilt.exe".into())
        );
    }
}
