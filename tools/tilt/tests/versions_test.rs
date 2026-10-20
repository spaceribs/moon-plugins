use proto_pdk_test_utils::*;

mod tilt_tool {
    use super::*;

    generate_resolve_versions_tests!("tilt-test", {
        "0.33" => "0.33.22",
        "0.37.8" => "0.37.8",
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git_tags() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox.create_plugin("tilt-test").await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
        assert!(output.latest.is_some());
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }
}
