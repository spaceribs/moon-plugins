# Spec Delta

## MODIFIED Requirements

### Requirement: Built-in and custom templates
The extension SHALL render each generated file from a built-in section template, registered as `moon/sections/<section>.tera`, with macros in `moon/macros.tera`. A `<section>.tera` file in `templateDir`, or in the directory passed with `generate --template-dir`, SHALL replace that section's template. The flag SHALL take precedence over the setting. A configured directory that doesn't exist, or that contains a `.tera` file naming no section, MUST be an error.

#### Scenario: Missing custom template
- **WHEN** `templateDir` points to a directory that doesn't exist
- **THEN** generation fails with an error naming the path

#### Scenario: Unknown section template
- **WHEN** `templateDir` contains `resource.tera` instead of `resources.tera`
- **THEN** generation fails with an error naming the file and the valid section names

### Requirement: Templates can extend the built-in one
A section template in `templateDir` SHALL be able to `{% extends "moon/sections/<section>.tera" %}` and override individual blocks, or import `moon/macros.tera` to reuse resource rendering.

#### Scenario: Override the footer block
- **WHEN** `templateDir/resources.tera` extends `moon/sections/resources.tera` and overrides only its `footer` block
- **THEN** `resources.tilt` is the built-in section plus the footer content, and every other file is unchanged
