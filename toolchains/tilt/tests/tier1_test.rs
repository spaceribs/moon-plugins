use moon_pdk_test_utils::*;

mod tilt_toolchain {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_toolchain("tilt").await;

        assert_eq!(plugin.metadata.name, "Tilt");
        assert_eq!(plugin.metadata.exe_names, ["tilt"]);
        assert!(plugin.metadata.config_file_globs.is_empty());
        assert_eq!(plugin.metadata.plugin_version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn bundles_the_proto_tool() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_toolchain("tilt").await;

        assert!(plugin.tool.is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn doesnt_infer_per_project_tasks() {
        // One Tilt instance per workspace: projects are selected through the
        // Tilt extension (`moon ext tilt -- up <project>`), not per-project
        // `tilt up` tasks.
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_toolchain("tilt").await;

        assert!(!plugin.plugin.has_func("extend_project_graph").await);
    }
}
