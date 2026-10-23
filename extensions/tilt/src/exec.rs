//! `moon ext tilt -- exec`: run a moon task where Tilt runs its project, in
//! its first declared workload or on the host. What runs where is decided
//! here from moon's and Tilt's query output; the extension runs it.

use crate::config::TiltExtensionConfig;
use crate::query::{QueryProjectsOutput, QueryTasksOutput};
use crate::workloads::declared_workloads;
use serde_json::Value;

/// Where a task runs.
#[derive(Debug, PartialEq)]
pub enum Location {
    /// On the host, from the workspace root.
    Host,
    /// In a running pod of this Kubernetes workload (Tilt resource name).
    Kubernetes(String),
    /// In the container of this Compose service (Tilt resource name).
    Compose(String),
}

/// A resolved `exec`: the task's target and where to run it.
#[derive(Debug, PartialEq)]
pub struct Exec {
    pub target: String,
    pub location: Location,
}

/// Resolve `target` (`<project>:<task>`) to where it runs, refusing tasks
/// tagged `confirmTag` without `yes`.
pub fn resolve(
    projects: &QueryProjectsOutput,
    tasks: &QueryTasksOutput,
    config: &TiltExtensionConfig,
    target: &str,
    yes: bool,
) -> Result<Exec, String> {
    let Some((project_id, task_id)) = target
        .split_once(':')
        .filter(|(project, task)| !project.is_empty() && !task.is_empty() && !task.contains(':'))
    else {
        return Err(format!(
            "<id>{target}</id> isn't a task target: use <id><project>:<task></id>"
        ));
    };
    let Some(project) = projects.projects.iter().find(|p| p.id == project_id) else {
        return Err(format!("No project <id>{project_id}</id> in the workspace"));
    };
    let Some(task) = tasks
        .tasks
        .get(project_id)
        .and_then(|tasks| tasks.get(task_id))
    else {
        return Err(format!(
            "Project <id>{project_id}</id> has no task <id>{task_id}</id>"
        ));
    };

    if task.tags.contains(&config.confirm_tag) && !yes {
        return Err(format!(
            "<id>{}</id> is tagged <id>{}</id> (destructive): pass <shell>--yes</shell> to run it",
            task.target, config.confirm_tag
        ));
    }

    let workloads = declared_workloads(project);
    let location = if let Some(name) = workloads.k8s.first() {
        Location::Kubernetes(name.clone())
    } else if let Some(name) = workloads.compose.first() {
        Location::Compose(name.clone())
    } else {
        Location::Host
    };

    Ok(Exec {
        target: task.target.clone(),
        location,
    })
}

/// Why `name` can't be an environment variable a button or `--env` sets, if
/// it can't: names are letters, digits, and underscores, not starting with a
/// digit (what a POSIX shell expands as `$NAME`), and `MOON_TILT_ARGS`
/// belongs to a button's input.
pub fn env_name_problem(name: &str) -> Option<&'static str> {
    let valid = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');

    if !valid {
        Some("must be letters, digits, and underscores, not starting with a digit")
    } else if name == crate::buttons::ARGS_ENV {
        Some("is set by a button's input")
    } else {
        None
    }
}

/// Environment variables from `--env KEY=VALUE` flags, in order. The value
/// is everything after the first `=`.
pub fn parse_env(flags: &[String]) -> Result<Vec<(String, String)>, String> {
    flags
        .iter()
        .map(|flag| {
            let Some((name, value)) = flag.split_once('=') else {
                return Err(format!(
                    "<shell>--env {flag}</shell> needs a value: use <shell>--env {flag}=<value></shell>"
                ));
            };

            match env_name_problem(name) {
                Some(problem) => Err(format!(
                    "<shell>--env {flag}</shell>: <property>{name}</property> {problem}"
                )),
                None => Ok((name.to_owned(), value.to_owned())),
            }
        })
        .collect()
}

/// The task's arguments: those after `--` (a button's argv, or a terminal's),
/// then the value of a button's input (`MOON_TILT_ARGS`), split into words.
pub fn task_args(mut args: Vec<String>, input: Option<&str>) -> Result<Vec<String>, String> {
    if let Some(value) = input {
        args.extend(split_args(value).map_err(|error| {
            format!(
                "Invalid <property>{}</property> (<shell>{value}</shell>): {error}",
                crate::buttons::ARGS_ENV
            )
        })?);
    }

    Ok(args)
}

/// `moon run` arguments for `target`, with `args` passed after `--`.
pub fn moon_run_args(target: &str, args: &[String]) -> Vec<String> {
    let mut run = vec!["run".to_owned(), target.to_owned()];

    if !args.is_empty() {
        run.push("--".into());
        run.extend(args.iter().cloned());
    }

    run
}

/// `kubectl` arguments that run `moon run <target>` in a running pod of
/// `workload`, from Tilt's `KubernetesDiscovery` for it and the `Cluster` it
/// deploys to, so the call reaches the cluster Tilt uses. `kubectl exec` has
/// no environment option, so variables in `env` are set with `env`.
pub fn kubectl_args(
    discovery: &Value,
    cluster: &Value,
    workload: &str,
    target: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<Vec<String>, String> {
    let pod = discovery["status"]["pods"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|pod| pod["phase"] == "Running" && pod["deleting"] != true)
        .ok_or_else(|| format!("Workload <id>{workload}</id> has no running pod in Tilt"))?;
    let (Some(name), Some(namespace)) = (pod["name"].as_str(), pod["namespace"].as_str()) else {
        return Err(format!(
            "Tilt reported a pod of <id>{workload}</id> without a name or namespace"
        ));
    };
    let kubernetes = &cluster["status"]["connection"]["kubernetes"];
    let mut kubectl = vec![];

    for (flag, value) in [
        ("--kubeconfig", &kubernetes["configPath"]),
        ("--context", &kubernetes["context"]),
    ] {
        if let Some(value) = value.as_str().filter(|value| !value.is_empty()) {
            kubectl.push(flag.to_owned());
            kubectl.push(value.to_owned());
        }
    }

    kubectl.extend(["exec", "-n", namespace, name, "--"].map(String::from));

    if !env.is_empty() {
        kubectl.push("env".into());
        kubectl.extend(env.iter().map(|(name, value)| format!("{name}={value}")));
    }

    kubectl.push("moon".into());
    kubectl.extend(moon_run_args(target, args));

    Ok(kubectl)
}

/// `docker` arguments that run `moon run <target>` in the running container
/// of Compose service `workload`, from Tilt's `DockerComposeService` for it,
/// with the variables in `env` set.
pub fn docker_args(
    service: &Value,
    workload: &str,
    target: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<Vec<String>, String> {
    let status = &service["status"];
    let container = status["containerName"]
        .as_str()
        .filter(|name| !name.is_empty() && status["containerState"]["running"] == true)
        .ok_or_else(|| {
            format!("Compose service <id>{workload}</id> has no running container in Tilt")
        })?;
    let mut docker = vec!["exec".to_owned()];

    for (name, value) in env {
        docker.push("-e".into());
        docker.push(format!("{name}={value}"));
    }

    docker.extend([container.to_owned(), "moon".to_owned()]);

    docker.extend(moon_run_args(target, args));

    Ok(docker)
}

/// Split a button input's value into arguments, the same on every platform:
/// whitespace separates words, single quotes are literal, double quotes allow
/// `\"` and `\\`, and a backslash outside quotes escapes the next character.
/// Nothing is expanded.
pub fn split_args(input: &str) -> Result<Vec<String>, String> {
    let mut args = vec![];
    let mut current: Option<String> = None;
    let mut chars = input.chars();

    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                args.extend(current.take());
            }
            '\'' => {
                let word = current.get_or_insert_with(String::new);

                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => word.push(c),
                        None => return Err("unclosed single quote".into()),
                    }
                }
            }
            '"' => {
                let word = current.get_or_insert_with(String::new);

                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(c @ ('"' | '\\')) => word.push(c),
                            Some(c) => {
                                word.push('\\');
                                word.push(c);
                            }
                            None => return Err("unclosed double quote".into()),
                        },
                        Some(c) => word.push(c),
                        None => return Err("unclosed double quote".into()),
                    }
                }
            }
            '\\' => {
                let word = current.get_or_insert_with(String::new);

                match chars.next() {
                    Some(c) => word.push(c),
                    None => word.push('\\'),
                }
            }
            c => current.get_or_insert_with(String::new).push(c),
        }
    }

    args.extend(current);

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::QueryProjectMetadata;

    fn fixture() -> (QueryProjectsOutput, QueryTasksOutput) {
        let projects =
            serde_json::from_str(include_str!("../tests/__fixtures__/query/projects.json"))
                .unwrap();
        let tasks =
            serde_json::from_str(include_str!("../tests/__fixtures__/query/tasks.json")).unwrap();

        (projects, tasks)
    }

    fn tilt_fixture(name: &str) -> Value {
        let json = match name {
            "kubernetesdiscovery" => {
                include_str!("../tests/__fixtures__/tilt/kubernetesdiscovery.json")
            }
            "dockercomposeservice" => {
                include_str!("../tests/__fixtures__/tilt/dockercomposeservice.json")
            }
            _ => include_str!("../tests/__fixtures__/tilt/cluster.json"),
        };

        serde_json::from_str(json).unwrap()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn resolve_target(
        projects: &QueryProjectsOutput,
        tasks: &QueryTasksOutput,
        target: &str,
        yes: bool,
    ) -> Result<Exec, String> {
        resolve(
            projects,
            tasks,
            &TiltExtensionConfig::default(),
            target,
            yes,
        )
    }

    #[test]
    fn resolves_where_a_task_runs() {
        let (mut projects, tasks) = fixture();

        assert_eq!(
            resolve_target(&projects, &tasks, "web:lint", false),
            Ok(Exec {
                target: "web:lint".into(),
                location: Location::Host,
            })
        );

        let api = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "api")
            .unwrap();
        api.config.project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(
                r#"{ "tilt": { "workloads": { "compose": ["api-cache"], "k8s": ["api"] } } }"#,
            )
            .unwrap(),
        });

        assert_eq!(
            resolve_target(&projects, &tasks, "api:build", false)
                .unwrap()
                .location,
            Location::Kubernetes("api".into())
        );

        let api = projects
            .projects
            .iter_mut()
            .find(|p| p.id == "api")
            .unwrap();
        api.config.project = Some(QueryProjectMetadata {
            metadata: serde_json::from_str(
                r#"{ "tilt": { "workloads": { "compose": ["api"] } } }"#,
            )
            .unwrap(),
        });

        assert_eq!(
            resolve_target(&projects, &tasks, "api:build", false)
                .unwrap()
                .location,
            Location::Compose("api".into())
        );
    }

    #[test]
    fn reports_unknown_targets() {
        let (projects, tasks) = fixture();

        for (target, error) in [
            ("web", "<id>web</id> isn't a task target"),
            (":lint", "isn't a task target"),
            ("web:", "isn't a task target"),
            ("a:b:c", "isn't a task target"),
            ("nope:lint", "No project <id>nope</id>"),
            ("web:nope", "Project <id>web</id> has no task <id>nope</id>"),
        ] {
            let message = resolve_target(&projects, &tasks, target, false).unwrap_err();

            assert!(message.contains(error), "{target}: {message}");
        }
    }

    #[test]
    fn confirmed_tasks_need_yes() {
        let (projects, mut tasks) = fixture();
        // Without a button too: the tag alone marks the task destructive.
        tasks
            .tasks
            .get_mut("web")
            .unwrap()
            .get_mut("lint")
            .unwrap()
            .tags = vec!["tilt-confirm".into()];

        assert_eq!(
            resolve_target(&projects, &tasks, "web:lint", false).unwrap_err(),
            "<id>web:lint</id> is tagged <id>tilt-confirm</id> (destructive): pass <shell>--yes</shell> to run it"
        );
        assert!(resolve_target(&projects, &tasks, "web:lint", true).is_ok());
    }

    #[test]
    fn builds_moon_run_args() {
        assert_eq!(moon_run_args("web:lint", &[]), ["run", "web:lint"]);
        assert_eq!(
            moon_run_args("web:lint", &strings(&["--fix", "src"])),
            ["run", "web:lint", "--", "--fix", "src"]
        );
    }

    #[test]
    fn splits_args_like_a_shell_without_expanding() {
        for (input, expected) in [
            ("", &[][..]),
            ("   ", &[][..]),
            ("--noinput --plan", &["--noinput", "--plan"][..]),
            ("  a\tb\n c ", &["a", "b", "c"][..]),
            ("'a b' \"c d\"", &["a b", "c d"][..]),
            ("--name='my app'", &["--name=my app"][..]),
            (r#""say \"hi\"" 'it''s'"#, &["say \"hi\"", "its"][..]),
            (r"a\ b c\\d", &["a b", r"c\d"][..]),
            (r#""\n" ''"#, &[r"\n", ""][..]),
            ("$HOME %PATH% *", &["$HOME", "%PATH%", "*"][..]),
        ] {
            assert_eq!(split_args(input).unwrap(), expected, "{input:?}");
        }

        assert_eq!(split_args("'open").unwrap_err(), "unclosed single quote");
        assert_eq!(split_args("\"open").unwrap_err(), "unclosed double quote");
    }

    #[test]
    fn passes_input_args_after_the_command_line_args() {
        // Only after `--`, as from a terminal or a button's argv.
        assert_eq!(task_args(strings(&["a"]), None).unwrap(), ["a"]);
        // Only a button's input.
        assert_eq!(task_args(vec![], Some("b 'c d'")).unwrap(), ["b", "c d"]);
        // Both: a button with argv and an input.
        assert_eq!(
            task_args(strings(&["--noinput"]), Some("--plan")).unwrap(),
            ["--noinput", "--plan"]
        );
        // An empty input adds nothing.
        assert_eq!(task_args(strings(&["a"]), Some("")).unwrap(), ["a"]);
        assert_eq!(
            task_args(strings(&["a"]), Some("'open")).unwrap_err(),
            "Invalid <property>MOON_TILT_ARGS</property> (<shell>'open</shell>): unclosed single quote"
        );
    }

    #[test]
    fn builds_kubectl_exec_from_tilts_discovery() {
        let discovery = tilt_fixture("kubernetesdiscovery");
        let cluster = tilt_fixture("cluster");

        assert_eq!(
            kubectl_args(
                &discovery,
                &cluster,
                "api",
                "api:db-migrate",
                &strings(&["--noinput"]),
                &[]
            )
            .unwrap(),
            [
                "--kubeconfig",
                "/tmp/tilt-dev/cluster/default.yml",
                "--context",
                "kind-kind",
                "exec",
                "-n",
                "moon-tilt-exec-probe",
                "probe-api-6df776c848-j7t4r",
                "--",
                "moon",
                "run",
                "api:db-migrate",
                "--",
                "--noinput",
            ]
        );

        // Without Tilt's kubeconfig or context, kubectl's own defaults apply.
        assert_eq!(
            kubectl_args(&discovery, &Value::Null, "api", "api:db-migrate", &[], &[]).unwrap()[..4],
            [
                "exec",
                "-n",
                "moon-tilt-exec-probe",
                "probe-api-6df776c848-j7t4r"
            ]
        );
    }

    #[test]
    fn kubectl_exec_needs_a_running_pod() {
        let mut discovery = tilt_fixture("kubernetesdiscovery");
        let cluster = tilt_fixture("cluster");
        let error = "Workload <id>api</id> has no running pod in Tilt";

        discovery["status"]["pods"][0]["phase"] = "Pending".into();
        assert_eq!(
            kubectl_args(&discovery, &cluster, "api", "api:x", &[], &[]).unwrap_err(),
            error
        );

        discovery["status"]["pods"][0]["phase"] = "Running".into();
        discovery["status"]["pods"][0]["deleting"] = true.into();
        assert_eq!(
            kubectl_args(&discovery, &cluster, "api", "api:x", &[], &[]).unwrap_err(),
            error
        );

        assert_eq!(
            kubectl_args(
                &serde_json::json!({ "status": {} }),
                &cluster,
                "api",
                "api:x",
                &[],
                &[]
            )
            .unwrap_err(),
            error
        );
    }

    #[test]
    fn builds_docker_exec_from_tilts_compose_service() {
        let mut service = tilt_fixture("dockercomposeservice");

        assert_eq!(
            docker_args(&service, "cache", "api:warm", &strings(&["--all"]), &[]).unwrap(),
            [
                "exec",
                "tilt-exec-probe-cache-1",
                "moon",
                "run",
                "api:warm",
                "--",
                "--all"
            ]
        );

        service["status"]["containerState"]["running"] = false.into();
        assert_eq!(
            docker_args(&service, "cache", "api:warm", &[], &[]).unwrap_err(),
            "Compose service <id>cache</id> has no running container in Tilt"
        );
    }

    #[test]
    fn parses_env_flags() {
        assert_eq!(parse_env(&[]).unwrap(), []);
        assert_eq!(
            parse_env(&strings(&["FLUSH_FLAGS=--noinput", "A=x y=z", "EMPTY="])).unwrap(),
            [
                ("FLUSH_FLAGS".to_owned(), "--noinput".to_owned()),
                ("A".to_owned(), "x y=z".to_owned()),
                ("EMPTY".to_owned(), String::new()),
            ]
        );
        assert_eq!(
            parse_env(&strings(&["FLUSH_FLAGS"])).unwrap_err(),
            "<shell>--env FLUSH_FLAGS</shell> needs a value: use <shell>--env FLUSH_FLAGS=<value></shell>"
        );
        assert_eq!(
            parse_env(&strings(&["FLUSH-FLAGS=x"])).unwrap_err(),
            "<shell>--env FLUSH-FLAGS=x</shell>: <property>FLUSH-FLAGS</property> must be letters, digits, and underscores, not starting with a digit"
        );
        assert_eq!(
            parse_env(&strings(&["=x"])).unwrap_err(),
            "<shell>--env =x</shell>: <property></property> must be letters, digits, and underscores, not starting with a digit"
        );
        assert_eq!(
            parse_env(&strings(&["MOON_TILT_ARGS=x"])).unwrap_err(),
            "<shell>--env MOON_TILT_ARGS=x</shell>: <property>MOON_TILT_ARGS</property> is set by a button's input"
        );
    }

    #[test]
    fn sets_env_in_workload_commands() {
        let discovery = tilt_fixture("kubernetesdiscovery");
        let service = tilt_fixture("dockercomposeservice");
        let env = [
            ("FLUSH_FLAGS".to_owned(), "--noinput".to_owned()),
            ("A".to_owned(), "x y=z".to_owned()),
        ];

        assert_eq!(
            kubectl_args(
                &discovery,
                &Value::Null,
                "api",
                "api:seed",
                &strings(&["--quick"]),
                &env
            )
            .unwrap(),
            [
                "exec",
                "-n",
                "moon-tilt-exec-probe",
                "probe-api-6df776c848-j7t4r",
                "--",
                "env",
                "FLUSH_FLAGS=--noinput",
                "A=x y=z",
                "moon",
                "run",
                "api:seed",
                "--",
                "--quick",
            ]
        );
        assert_eq!(
            docker_args(&service, "cache", "api:seed", &[], &env).unwrap(),
            [
                "exec",
                "-e",
                "FLUSH_FLAGS=--noinput",
                "-e",
                "A=x y=z",
                "tilt-exec-probe-cache-1",
                "moon",
                "run",
                "api:seed",
            ]
        );
    }
}
