# tilt-templates Specification

## Purpose
How the generated Tiltfile is rendered with Tera templates, and how users customize it by extending or replacing the built-in template.

## Requirements

### Requirement: Built-in and custom templates
The extension SHALL render the generated file with its built-in Tera template, registered as `moon/Tiltfile.moon.tera` with macros in `moon/macros.tera`, unless a custom template is configured with `template` or passed with `generate --template`. The flag SHALL take precedence over the setting, and a configured template that doesn't exist MUST be an error.

#### Scenario: Missing custom template
- **WHEN** `template` points to a file that doesn't exist
- **THEN** generation fails with an error naming the path

### Requirement: Templates can extend the built-in one
A custom template SHALL be able to `{% extends "moon/Tiltfile.moon.tera" %}` and override individual blocks, or import `moon/macros.tera` to reuse resource rendering.

#### Scenario: Override the footer block
- **WHEN** a custom template extends the built-in one and overrides only `footer`
- **THEN** the output is the built-in file plus the footer content

### Requirement: Starlark-safe filters
Templates SHALL have a `starlark` filter that renders any value as a Starlark literal, and a `moon_path` filter that wraps workspace-relative paths in `_moon_path()`, keeping `!`-prefixed patterns negated.

#### Scenario: starlark filter
- **WHEN** a template renders `{{ ["a", true] | starlark }}`
- **THEN** the output is `["a", True]`

### Requirement: Render errors name the template
A template that fails to render MUST fail generation with an error naming the template and Tera's underlying cause.

#### Scenario: Unknown variable
- **WHEN** a custom template references an undefined variable
- **THEN** generation fails with an error naming the template and the variable
