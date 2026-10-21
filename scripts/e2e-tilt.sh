#!/usr/bin/env bash
# End-to-end check of the Tilt extension and toolchain with real moon and Tilt,
# on examples/tilt-demo. Runs on Linux, macOS, and Windows (Git Bash) in CI:
#
#   1. builds the plugins, installs the toolchain's Tilt (`moon setup`), and
#      generates the Tiltfile
#   2. `moon ext tilt -- ci web`: web's resources and their dependencies come up
#   3. `moon ext tilt -- up web`, then edits files and checks that only what's
#      affected re-runs: a non-input or gitignored file re-runs nothing, and an
#      input re-runs its task
#
# Needs moon and Node on PATH. Tilt comes from the demo's Tilt toolchain, so
# this also checks that the extension runs the toolchain's installed binary.

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
demo="$root/examples/tilt-demo"
proto_home="${PROTO_HOME:-$HOME/.proto}"
# Away from the default port, so a developer's own Tilt isn't touched.
export TILT_PORT="${TILT_PORT:-10351}"
# Seconds to wait for a re-run that must not happen.
settle="${E2E_SETTLE_SECONDS:-15}"

step() { printf '\n==> %s\n' "$*"; }
fail() { printf '\nFAIL: %s\n' "$*" >&2; exit 1; }

step "Build the plugins"
(cd "$root" && moon run tilt-extension:build tilt-toolchain:build)

cd "$demo"

step "Install the toolchain's Tilt"
moon setup

# The toolchain's installed binary, for querying the running Tilt.
tilt=""
for candidate in "$proto_home"/tools/tilt/*/tilt "$proto_home"/tools/tilt/*/tilt.exe; do
  [ -f "$candidate" ] && tilt="$candidate"
done
[ -n "$tilt" ] || fail "moon setup didn't install Tilt under $proto_home/tools/tilt"
echo "Tilt: $tilt"

step "Generate the Tiltfile"
moon ext tilt -- generate

step "tilt ci: web and its dependencies come up"
moon ext tilt -- ci web

step "tilt up web"
log="$demo/.moon/cache/e2e-tilt-up.log"
mkdir -p "$(dirname "$log")"
moon ext tilt -- up web -- --stream > "$log" 2>&1 &
up_pid=$!

cleanup() {
  "$tilt" down --file Tiltfile > /dev/null 2>&1 || true
  kill "$up_pid" 2> /dev/null || true
  # Killing moon doesn't stop the Tilt it started. The toolchain's binary path
  # is specific to this run's Tilt.
  if command -v pkill > /dev/null 2>&1; then
    pkill -f "$tilt up" 2> /dev/null || true
  elif command -v taskkill > /dev/null 2>&1; then
    taskkill //F //IM tilt.exe > /dev/null 2>&1 || true
  fi
  rm -rf web/node_modules/e2e-ignored.js web/notes.txt
  git -C "$demo" checkout -- web/schema.json 2> /dev/null || true
}
trap cleanup EXIT

# Builds of a resource so far (each run of a build step, or restart of a service).
builds() {
  "$tilt" get uiresource "$1" -o json |
    node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>console.log((JSON.parse(s).status.buildHistory||[]).length))'
}

# Wait for Tilt to start and the resources to be ready.
for _ in $(seq 1 60); do
  "$tilt" get session > /dev/null 2>&1 && break
  sleep 2
done
"$tilt" wait --for=condition=Ready uiresource/web:codegen uiresource/web:dev --timeout=300s ||
  { cat "$log"; fail "web's resources didn't become ready"; }

snapshot() { echo "codegen=$(builds web:codegen) dev=$(builds web:dev)"; }

# Edits that must not re-run anything.
expect_no_rerun() {
  local what=$1 before after
  before=$(snapshot)
  "${@:2}"
  sleep "$settle"
  after=$(snapshot)
  [ "$before" = "$after" ] || { cat "$log"; fail "$what re-ran resources ($before -> $after)"; }
  echo "ok: $what re-ran nothing ($after)"
}

step "Edit a file that isn't an input"
expect_no_rerun "a non-input file" sh -c 'echo "notes $(date +%s)" > web/notes.txt'

step "Edit a gitignored file"
expect_no_rerun "a gitignored file" sh -c 'mkdir -p web/node_modules && echo "// $(date +%s)" > web/node_modules/e2e-ignored.js'

step "Edit an input"
before=$(builds web:codegen)
node -e 'require("fs").writeFileSync("web/schema.json", JSON.stringify([Date.now()]) + "\n")'
for _ in $(seq 1 60); do
  [ "$(builds web:codegen)" -gt "$before" ] && break
  sleep 2
done
after=$(builds web:codegen)
[ "$after" -gt "$before" ] || { cat "$log"; fail "editing an input didn't re-run web:codegen ($before -> $after)"; }
echo "ok: editing web/schema.json re-ran web:codegen ($before -> $after)"

step "Passed"
