//! `moon ext tilt -- generate --load`: whether Tilt can load the main
//! Tiltfile, from `tilt alpha tiltfile-result`, which evaluates it without a
//! cluster or a running Tilt.

/// `tilt` arguments that load `tiltfile` and report the result.
pub fn tiltfile_result_args(tiltfile: &str) -> Vec<String> {
    ["alpha", "tiltfile-result", "--file", tiltfile]
        .map(String::from)
        .into()
}

/// The outcome of `tilt alpha tiltfile-result`, from its documented exit
/// codes: 0 loaded (the result JSON on stdout, ignored), 5 the Tiltfile
/// failed to evaluate, anything else a failure to run it. Errors carry Tilt's
/// own output.
pub fn load_result(
    tiltfile: &str,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> Result<(), String> {
    match exit_code {
        0 => Ok(()),
        5 => {
            // Tilt's framing, not part of the error.
            let error = stderr
                .lines()
                .filter(|line| !line.starts_with("Loading Tiltfile at:"))
                .collect::<Vec<_>>()
                .join("\n");

            Err(format!(
                "<path>{tiltfile}</path> doesn't load in Tilt:\n{}",
                error.trim()
            ))
        }
        code => {
            let output = if stderr.trim().is_empty() {
                stdout
            } else {
                stderr
            };

            Err(format!(
                "<shell>tilt alpha tiltfile-result</shell> exited with code {code}: {}",
                output.trim()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_quietly() {
        assert_eq!(
            tiltfile_result_args("Tiltfile").join(" "),
            "alpha tiltfile-result --file Tiltfile"
        );
        assert_eq!(
            load_result("Tiltfile", 0, "{\"Manifests\": []}", ""),
            Ok(())
        );
    }

    #[test]
    fn reports_tilts_error_when_the_tiltfile_fails() {
        // Captured from Tilt 0.37.0.
        let stderr = "Loading Tiltfile at: /workspace/Tiltfile\n\
                      /workspace/Tiltfile:1:13: k8s_resource specified unknown resource \"api\". known k8s resources: \n";

        assert_eq!(
            load_result("Tiltfile", 5, "", stderr).unwrap_err(),
            "<path>Tiltfile</path> doesn't load in Tilt:\n/workspace/Tiltfile:1:13: k8s_resource specified unknown resource \"api\". known k8s resources:"
        );
    }

    #[test]
    fn reports_other_failures_with_their_code() {
        assert_eq!(
            load_result("Tiltfile", 1, "", "Error: unknown flag: --nope\n").unwrap_err(),
            "<shell>tilt alpha tiltfile-result</shell> exited with code 1: Error: unknown flag: --nope"
        );
        assert_eq!(
            load_result("Tiltfile", 2, "partial\n", "").unwrap_err(),
            "<shell>tilt alpha tiltfile-result</shell> exited with code 2: partial"
        );
    }
}
