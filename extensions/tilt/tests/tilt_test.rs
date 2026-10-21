use moon_pdk_test_utils::*;
use serde_json::json;

/// A fake `moon` that replays captured `moon query` output (`<command>.json`
/// next to it), as the extension's `moonBin`.
#[cfg(unix)]
const FAKE_MOON: &str = "/workspace/fake-moon";
#[cfg(windows)]
const FAKE_MOON: &str = "/workspace/fake-moon.cmd";

fn create_fake_moon(sandbox: &MoonWasmSandbox) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        sandbox.create_file(
            "fake-moon",
            "#!/bin/sh\ncat \"$(dirname \"$0\")/$2.json\"\n",
        );

        let path = sandbox.path().join("fake-moon");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(windows)]
    sandbox.create_file("fake-moon.cmd", "@type \"%~dp0%2.json\"\r\n");
}

mod tilt_extension {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn registers_metadata() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("tilt").await;

        assert_eq!(plugin.metadata.name, "Tilt");
        assert_eq!(plugin.metadata.plugin_version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "A command is required")]
    async fn errors_without_a_command() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("tilt").await;

        plugin
            .execute_extension(ExecuteExtensionInput::default())
            .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    #[should_panic(expected = "unrecognized subcommand")]
    async fn errors_on_unknown_command() {
        let sandbox = create_empty_moon_sandbox();
        let plugin = sandbox.create_extension("tilt").await;

        plugin
            .execute_extension(ExecuteExtensionInput {
                args: vec!["unknown".into()],
                ..Default::default()
            })
            .await;
    }

    mod generate {
        use super::*;

        pub async fn create_plugin(sandbox: &MoonWasmSandbox) -> ExtensionTestWrapper {
            create_fake_moon(sandbox);

            sandbox
                .create_extension_with_config("tilt", |config| {
                    config.insert("moon_extension_config", json!({ "moonBin": FAKE_MOON }));
                })
                .await
        }

        fn generate_args(extra: &[&str]) -> ExecuteExtensionInput {
            let mut args = vec!["generate".to_owned()];
            args.extend(extra.iter().map(|arg| arg.to_string()));

            ExecuteExtensionInput {
                args,
                ..Default::default()
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn writes_generated_and_main_tiltfiles() {
            let sandbox = create_moon_sandbox("query");
            let plugin = create_plugin(&sandbox).await;

            plugin.execute_extension(generate_args(&[])).await;

            let generated = std::fs::read_to_string(sandbox.path().join("Tiltfile.moon")).unwrap();

            assert!(generated.contains("local_resource(\n    \"web:dev\","));
            assert!(generated.contains(&format!("\"{FAKE_MOON}\", \"exec\", \"api:serve\"")));
            assert!(!generated.contains("web:lint"));

            let main = std::fs::read_to_string(sandbox.path().join("Tiltfile")).unwrap();

            assert!(main.contains("include(\"./Tiltfile.moon\")"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_overwrite_existing_main_tiltfile() {
            let sandbox = create_moon_sandbox("query");
            sandbox.create_file("Tiltfile", "# mine\n");
            let plugin = create_plugin(&sandbox).await;

            plugin.execute_extension(generate_args(&[])).await;

            assert_eq!(
                std::fs::read_to_string(sandbox.path().join("Tiltfile")).unwrap(),
                "# mine\n"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn check_passes_when_up_to_date() {
            let sandbox = create_moon_sandbox("query");
            let plugin = create_plugin(&sandbox).await;

            plugin.execute_extension(generate_args(&[])).await;
            plugin.execute_extension(generate_args(&["--check"])).await;
        }

        /// Rewrite the captured `moon query projects` output so `api` still
        /// sets the removed `project.tilt.k8s`.
        pub fn set_removed_k8s_settings(sandbox: &MoonWasmSandbox) {
            let path = sandbox.path().join("projects.json");
            let mut projects: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let api = projects["projects"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|project| project["id"] == "api")
                .unwrap();

            api["config"]["project"] =
                json!({ "metadata": { "tilt": { "k8s": { "ports": [3001] } } } });
            std::fs::write(path, projects.to_string()).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "names projects that don't exist")]
        async fn fails_for_unknown_runtime_dependencies() {
            let sandbox = create_moon_sandbox("query");
            let path = sandbox.path().join("projects.json");
            let mut projects: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let web = projects["projects"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|project| project["id"] == "web")
                .unwrap();

            web["config"]["project"] = json!({ "metadata": { "tilt": { "dependsOn": ["apii"] } } });
            std::fs::write(path, projects.to_string()).unwrap();
            let plugin = create_plugin(&sandbox).await;

            plugin.execute_extension(generate_args(&[])).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "export-k8s api")]
        async fn fails_with_migration_guidance_for_k8s_settings() {
            let sandbox = create_moon_sandbox("query");
            set_removed_k8s_settings(&sandbox);
            let plugin = create_plugin(&sandbox).await;

            plugin.execute_extension(generate_args(&[])).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "is out of date")]
        async fn check_fails_when_stale() {
            let sandbox = create_moon_sandbox("query");
            sandbox.create_file("Tiltfile.moon", "# stale\n");
            let plugin = create_plugin(&sandbox).await;

            plugin.execute_extension(generate_args(&["--check"])).await;
        }
    }

    mod export_k8s {
        use super::*;

        async fn export(sandbox: &MoonWasmSandbox, extra: &[&str]) {
            let plugin = super::generate::create_plugin(sandbox).await;
            let mut args = vec!["export-k8s".to_owned(), "api".to_owned()];
            args.extend(extra.iter().map(|arg| arg.to_string()));

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args,
                    ..Default::default()
                })
                .await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn writes_the_manifest_next_to_the_project() {
            let sandbox = create_moon_sandbox("query");
            super::generate::set_removed_k8s_settings(&sandbox);

            export(&sandbox, &[]).await;

            let manifest = std::fs::read_to_string(sandbox.path().join("api/k8s.yaml")).unwrap();

            assert!(manifest.starts_with("apiVersion: apps/v1\nkind: Deployment\n"));
            assert!(manifest.contains("kind: Service\n"));
            assert!(manifest.contains("- containerPort: 3001\n"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn writes_to_a_custom_output_path() {
            let sandbox = create_moon_sandbox("query");
            super::generate::set_removed_k8s_settings(&sandbox);

            export(&sandbox, &["--output", "deploy/k8s/api.yaml"]).await;

            assert!(sandbox.path().join("deploy/k8s/api.yaml").is_file());
            assert!(!sandbox.path().join("api/k8s.yaml").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "already exists")]
        async fn refuses_to_overwrite() {
            let sandbox = create_moon_sandbox("query");
            super::generate::set_removed_k8s_settings(&sandbox);
            sandbox.create_file("api/k8s.yaml", "# mine\n");

            export(&sandbox, &[]).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn leaves_an_existing_file_unchanged() {
            let sandbox = create_moon_sandbox("query");
            super::generate::set_removed_k8s_settings(&sandbox);
            sandbox.create_file("api/k8s.yaml", "# mine\n");
            let plugin = super::generate::create_plugin(&sandbox).await;

            let result = plugin
                .plugin
                .call_func_without_output(
                    "execute_extension",
                    ExecuteExtensionInput {
                        args: vec!["export-k8s".into(), "api".into()],
                        context: plugin.create_context(),
                    },
                )
                .await;

            assert!(result.is_err());
            assert_eq!(
                std::fs::read_to_string(sandbox.path().join("api/k8s.yaml")).unwrap(),
                "# mine\n"
            );
        }
    }

    mod sync_workspace {
        use super::*;

        async fn sync(plugin: &ExtensionTestWrapper, config: serde_json::Value) -> SyncOutput {
            plugin
                .plugin
                .call_func_with(
                    "sync_workspace",
                    SyncWorkspaceInput {
                        context: plugin.create_context(),
                        extension_config: config,
                        ..Default::default()
                    },
                )
                .await
                .unwrap()
        }

        fn changed_names(output: &SyncOutput) -> Vec<String> {
            let mut names: Vec<_> = output
                .changed_files
                .iter()
                .map(|file| file.to_string().rsplit('/').next().unwrap().to_owned())
                .collect();
            names.sort();
            names
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn writes_files_then_reports_no_changes() {
            let sandbox = create_moon_sandbox("query");
            create_fake_moon(&sandbox);
            let plugin = sandbox.create_extension("tilt").await;
            let config = json!({ "moonBin": FAKE_MOON });

            let output = sync(&plugin, config.clone()).await;

            assert_eq!(changed_names(&output), ["Tiltfile", "Tiltfile.moon"]);
            assert_eq!(output.operations[0].status, OperationStatus::Passed);

            // Unchanged contents aren't rewritten, so Tilt doesn't reload.
            let output = sync(&plugin, config).await;

            assert!(output.changed_files.is_empty());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_project_tiltfiles() {
            let sandbox = create_moon_sandbox("query");
            create_fake_moon(&sandbox);
            sandbox.create_file("docs/Tiltfile", "local_resource('docs', 'echo')\n");
            let plugin = sandbox.create_extension("tilt").await;

            sync(&plugin, json!({ "moonBin": FAKE_MOON })).await;

            let generated = std::fs::read_to_string(sandbox.path().join("Tiltfile.moon")).unwrap();

            assert!(generated.contains("include(_moon_path(\"docs/Tiltfile\"))"));
            assert!(!generated.contains("web/Tiltfile"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_be_disabled() {
            let sandbox = create_moon_sandbox("query");
            let plugin = sandbox.create_extension("tilt").await;

            let output = sync(&plugin, json!({ "sync": false })).await;

            assert!(output.skipped);
            assert!(!sandbox.path().join("Tiltfile.moon").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn records_removed_k8s_settings_without_failing_the_pipeline() {
            let sandbox = create_moon_sandbox("query");
            create_fake_moon(&sandbox);
            super::generate::set_removed_k8s_settings(&sandbox);
            let plugin = sandbox.create_extension("tilt").await;

            let output = sync(&plugin, json!({ "moonBin": FAKE_MOON })).await;
            let error = output.operations[0].error.as_deref().unwrap_or_default();

            assert_eq!(output.operations[0].status, OperationStatus::Failed);
            assert!(error.contains("project.tilt.workloads"), "{error}");
            assert!(!sandbox.path().join("Tiltfile.moon").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn records_failures_without_failing_the_pipeline() {
            let sandbox = create_moon_sandbox("query");
            let plugin = sandbox.create_extension("tilt").await;

            let output = sync(&plugin, json!({ "moonBin": "/workspace/missing-moon" })).await;

            assert!(output.changed_files.is_empty());
            assert_eq!(output.operations[0].status, OperationStatus::Failed);
            assert!(output.operations[0].error.is_some());
        }
    }

    mod templates {
        use super::*;

        async fn create_plugin(
            sandbox: &MoonWasmSandbox,
            extra: serde_json::Value,
        ) -> ExtensionTestWrapper {
            create_fake_moon(sandbox);

            let mut config = json!({ "moonBin": FAKE_MOON });
            config
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());

            sandbox
                .create_extension_with_config("tilt", move |builder| {
                    builder.insert("moon_extension_config", config.clone());
                })
                .await
        }

        async fn generate(plugin: &ExtensionTestWrapper, args: &[&str]) {
            let mut input_args = vec!["generate".to_owned()];
            input_args.extend(args.iter().map(|arg| arg.to_string()));

            plugin
                .execute_extension(ExecuteExtensionInput {
                    args: input_args,
                    ..Default::default()
                })
                .await;
        }

        fn read_generated(sandbox: &MoonWasmSandbox) -> String {
            std::fs::read_to_string(sandbox.path().join("Tiltfile.moon")).unwrap()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn extends_the_built_in_template() {
            let sandbox = create_moon_sandbox("query");
            sandbox.create_file(
                "tilt/Tiltfile.tera",
                r#"{% extends "moon/Tiltfile.moon.tera" %}
{% block footer %}
{%- for project in projects %}{% if project.resources %}
k8s_yaml(_moon_path("{{ project.source }}/k8s"))
{%- endif %}{% endfor %}
{% endblock footer %}"#,
            );
            let plugin = create_plugin(&sandbox, json!({ "template": "tilt/Tiltfile.tera" })).await;

            generate(&plugin, &[]).await;

            let output = read_generated(&sandbox);

            assert!(output.starts_with("# Code generated by tilt_extension"));
            assert!(output.contains("local_resource(\n    \"web:dev\","));
            assert!(output.contains("k8s_yaml(_moon_path(\"api/k8s\"))"));
            assert!(output.contains("k8s_yaml(_moon_path(\"web/k8s\"))"));
            assert!(!output.contains("docs/k8s"));
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn replaces_the_built_in_template() {
            let sandbox = create_moon_sandbox("query");
            sandbox.create_file(
                "custom.tera",
                r#"{% import "moon/macros.tera" as moon -%}
# only services
{% for resource in resources | filter(attribute="persistent", value=true) -%}
{{ moon::resource(resource=resource) }}
{% endfor %}"#,
            );
            let plugin = create_plugin(&sandbox, json!({})).await;

            generate(&plugin, &["--template", "custom.tera"]).await;

            let output = read_generated(&sandbox);

            assert!(output.starts_with("# only services"));
            assert!(output.contains("\"web:dev\""));
            assert!(!output.contains("\"web:codegen\","));
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "does not exist")]
        async fn errors_when_template_is_missing() {
            let sandbox = create_moon_sandbox("query");
            let plugin = create_plugin(&sandbox, json!({ "template": "missing.tera" })).await;

            generate(&plugin, &[]).await;
        }

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Failed to render Tilt template")]
        async fn errors_when_template_is_invalid() {
            let sandbox = create_moon_sandbox("query");
            sandbox.create_file("bad.tera", "{{ unknown_variable }}");
            let plugin = create_plugin(&sandbox, json!({ "template": "bad.tera" })).await;

            generate(&plugin, &[]).await;
        }
    }
}
