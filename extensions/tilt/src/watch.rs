//! Translates moon's input/output semantics into Tilt file-watching rules, so
//! Tilt only re-runs a resource when moon would consider the task affected.
//!
//! Tilt's `deps` only accepts paths and its `ignore` accepts `.dockerignore`
//! patterns (last match wins, `!` re-includes). So each resource watches the
//! static base directories of its inputs and ignores everything in them except
//! the input globs, mirroring how moon hashes inputs:
//!
//! 1. `<base>/**`: ignore everything under each input glob's base directory
//! 2. `!<glob>`: re-include files matching the input globs
//! 3. gitignored paths: moon never hashes them (rendered globally)
//! 4. negated input globs and the task's own outputs
//! 5. `!<file>`: explicit input files, which moon hashes even when gitignored

use std::collections::BTreeSet;

/// Watch rules for a single resource. All paths are workspace-relative.
#[derive(Debug, Default, PartialEq)]
pub struct WatchRules {
    /// Paths for Tilt to watch (`deps`).
    pub watch: Vec<String>,
    /// Patterns applied before the gitignore patterns (steps 1-2).
    pub only_inputs: Vec<String>,
    /// Patterns applied after the gitignore patterns (steps 4-5).
    pub ignore: Vec<String>,
}

/// Build watch rules from a task's resolved inputs and outputs.
pub fn plan_watch<'a>(
    input_files: impl IntoIterator<Item = &'a String>,
    input_globs: impl IntoIterator<Item = &'a String>,
    output_files: impl IntoIterator<Item = &'a String>,
    output_globs: impl IntoIterator<Item = &'a String>,
) -> WatchRules {
    let mut watch = BTreeSet::new();
    let mut bases = BTreeSet::new();
    let mut include_globs = vec![];
    let mut exclude_globs = vec![];

    for glob in input_globs {
        if let Some(negated) = glob.strip_prefix('!') {
            exclude_globs.extend(expand_braces(negated));
            continue;
        }

        let base = glob_base(glob);

        // Watching the workspace root or moon's own config would re-trigger
        // every resource on any change, so those globs are not watched.
        if !is_watchable(&base) {
            continue;
        }

        bases.insert(base.clone());
        watch.insert(base);
        include_globs.extend(expand_braces(glob));
    }

    let files: Vec<&String> = input_files
        .into_iter()
        .filter(|file| is_watchable(file))
        .collect();

    watch.extend(files.iter().map(|file| file.to_string()));

    let mut only_inputs: Vec<String> = bases.iter().map(|base| format!("{base}/**")).collect();
    only_inputs.extend(include_globs.into_iter().map(|glob| format!("!{glob}")));

    let mut ignore = exclude_globs;

    for file in output_files {
        ignore.push(file.to_owned());
        ignore.push(format!("{file}/**"));
    }

    for glob in output_globs {
        ignore.extend(expand_braces(glob));
    }

    ignore.extend(files.iter().map(|file| format!("!{file}")));

    WatchRules {
        watch: collapse_nested(watch),
        only_inputs,
        ignore,
    }
}

/// Remove paths that are already covered by a watched parent directory.
fn collapse_nested(paths: BTreeSet<String>) -> Vec<String> {
    let mut kept: Vec<String> = vec![];

    // BTreeSet ordering guarantees parents sort before their children.
    for path in paths {
        if !kept
            .iter()
            .any(|parent| path.starts_with(&format!("{parent}/")))
        {
            kept.push(path);
        }
    }

    kept
}

fn is_watchable(path: &str) -> bool {
    !path.is_empty() && path != ".moon" && !path.starts_with(".moon/")
}

/// The static directory prefix of a glob, e.g. `app/src/**/*.ts` -> `app/src`.
pub fn glob_base(glob: &str) -> String {
    glob.split('/')
        .take_while(|part| !part.contains(['*', '?', '[', '{', '!']))
        .collect::<Vec<_>>()
        .join("/")
}

/// Expand `{a,b}` alternations (which `.dockerignore` patterns lack) into
/// separate patterns, e.g. `*.{ts,tsx}` -> `*.ts`, `*.tsx`. Supports nesting.
pub fn expand_braces(pattern: &str) -> Vec<String> {
    let Some(open) = pattern.find('{') else {
        return vec![pattern.to_owned()];
    };

    // Find the matching close brace and top-level commas.
    let mut depth = 0;
    let mut commas = vec![];
    let mut close = None;

    for (index, char) in pattern[open..].char_indices() {
        let index = open + index;

        match char {
            '{' => depth += 1,
            '}' => {
                depth -= 1;

                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            ',' if depth == 1 => commas.push(index),
            _ => {}
        }
    }

    let Some(close) = close else {
        // Unbalanced: leave as-is rather than guess.
        return vec![pattern.to_owned()];
    };

    let prefix = &pattern[..open];
    let suffix = &pattern[close + 1..];
    let mut bounds = vec![open];
    bounds.extend(commas);
    bounds.push(close);

    bounds
        .windows(2)
        .flat_map(|window| {
            let option = &pattern[window[0] + 1..window[1]];

            expand_braces(&format!("{prefix}{option}{suffix}"))
        })
        .collect()
}

/// Translate a `.gitignore` (located in workspace-relative `dir`) into
/// workspace-relative `.dockerignore`-style patterns for Tilt.
pub fn gitignore_patterns(dir: &str, contents: &str) -> Vec<String> {
    let dir = dir.trim_matches('/');
    let dir = if dir == "." { "" } else { dir };
    let mut patterns = vec![];

    for line in contents.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (negated, line) = match line.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, line.strip_prefix('\\').unwrap_or(line)),
        };

        // A trailing slash only means "directories", which `.dockerignore`
        // can't express; matching files of the same name too is harmless.
        let line = line.trim_end_matches('/');

        if line.is_empty() {
            continue;
        }

        // Patterns containing a slash are anchored to the .gitignore's
        // directory; others match at any depth.
        let body = if line.contains('/') {
            line.trim_start_matches('/').to_owned()
        } else {
            format!("**/{line}")
        };

        let full = if dir.is_empty() {
            body
        } else {
            format!("{dir}/{body}")
        };

        let prefix = if negated { "!" } else { "" };

        for expanded in expand_braces(&full) {
            patterns.push(format!("{prefix}{expanded}"));
            patterns.push(format!("{prefix}{expanded}/**"));
        }
    }

    patterns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn computes_glob_base() {
        assert_eq!(glob_base("app/src/**/*.ts"), "app/src");
        assert_eq!(glob_base("app/**/*"), "app");
        assert_eq!(glob_base("**/*"), "");
        assert_eq!(glob_base("app/{a,b}/x"), "app");
        assert_eq!(glob_base("app/package.json"), "app/package.json");
    }

    #[test]
    fn expands_braces() {
        assert_eq!(expand_braces("src/*.ts"), ["src/*.ts"]);
        assert_eq!(expand_braces("src/*.{ts,tsx}"), ["src/*.ts", "src/*.tsx"]);
        assert_eq!(
            expand_braces("{a,b}/*.{x,y}"),
            ["a/*.x", "a/*.y", "b/*.x", "b/*.y"]
        );
        assert_eq!(expand_braces("a/{b,{c,d}}"), ["a/b", "a/c", "a/d"]);
        assert_eq!(expand_braces("a/{b,c"), ["a/{b,c"]);
    }

    #[test]
    fn translates_gitignore() {
        let contents = "# comment\n\nnode_modules/\n/dist\nbuild/out\n*.log\n!keep.log\n\\#hash\n";

        assert_eq!(
            gitignore_patterns("", contents),
            strings(&[
                "**/node_modules",
                "**/node_modules/**",
                "dist",
                "dist/**",
                "build/out",
                "build/out/**",
                "**/*.log",
                "**/*.log/**",
                "!**/keep.log",
                "!**/keep.log/**",
                "**/#hash",
                "**/#hash/**",
            ])
        );

        assert_eq!(
            gitignore_patterns("apps/web", "/generated\n.turbo\n"),
            strings(&[
                "apps/web/generated",
                "apps/web/generated/**",
                "apps/web/**/.turbo",
                "apps/web/**/.turbo/**",
            ])
        );
    }

    #[test]
    fn plans_watch_rules_like_moon_hashes_inputs() {
        let rules = plan_watch(
            &strings(&["app/package.json", "tsconfig.base.json"]),
            &strings(&[
                "app/src/**/*.{ts,tsx}",
                "!app/src/**/*.test.ts",
                ".moon/*.yml",
                "**/*.graphql",
            ]),
            &strings(&["app/dist"]),
            &strings(&["app/src/**/*.d.ts"]),
        );

        assert_eq!(
            rules.watch,
            ["app/package.json", "app/src", "tsconfig.base.json"]
        );
        assert_eq!(
            rules.only_inputs,
            ["app/src/**", "!app/src/**/*.ts", "!app/src/**/*.tsx"]
        );
        assert_eq!(
            rules.ignore,
            [
                "app/src/**/*.test.ts",
                "app/dist",
                "app/dist/**",
                "app/src/**/*.d.ts",
                "!app/package.json",
                "!tsconfig.base.json",
            ]
        );
    }

    #[test]
    fn collapses_nested_watch_paths() {
        let rules = plan_watch(
            &strings(&["app/src/main.ts"]),
            &strings(&["app/**/*", "app/src/**/*"]),
            &[],
            &[],
        );

        assert_eq!(rules.watch, ["app"]);
    }
}
