# Proposal

## Why

The extension already leaves out tasks that run Tilt, but tasks that depend on one still become resources (issue #4). Their dependency becomes a `resource_deps` entry for a resource that doesn't exist, and Tilt drops it without an error. So a task meant to run *after* `tilt up` (for example `tilt-open`, which opens the Tilt UI) runs right away, inside the Tilt it was meant to follow. In the workspace that filed the issue, `moon ext tilt -- up` opened the browser 16 times.

## What Changes

- A task is also excluded when it depends on a task that runs Tilt, or on a task excluded for that reason (so chains like `tilt-up` → `tilt-open` → `notify` are all left out). Like tasks that run Tilt themselves, these are excluded even when tagged.
- The generated Tiltfile lists the tasks excluded this way, each with the dependency that caused it, so the exclusion isn't silent.
- No resource has a `resource_deps` entry for an excluded task anymore.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-resources`: "Tasks that cannot run under Tilt are excluded" now also covers tasks that depend on a task that runs Tilt (directly or through other excluded tasks), and requires the generated file to name them.

## Impact

- **Code**: `extensions/tilt/src/tiltfile.rs` (task selection and dependency walk), the built-in template (a comment listing excluded tasks), and the template context (a new variable for custom templates).
- **Behavior**: workspaces with such tasks lose resources that were running too early. This is a fix (`fix(tilt)`, patch release), so it's not opt-in. See design.md.
- **Docs**: extension README (which tasks are never exposed, and the new template variable).
