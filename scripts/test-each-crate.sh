#!/usr/bin/env bash
# Run every workspace crate's tests, one crate at a time, and name the ones
# that fail.
#
# The crate list comes from `cargo metadata`, not a hand-kept list: CI used
# to name five crates by hand and the other fourteen - knot-processes among
# them - never ran their tests on Linux, which is how a `kill` that signals
# every process the user owns reached a release (hotfix 1.24.1). A new crate
# is tested the day it is added.
#
# One crate per `cargo test -p` so a failure or a hang points at its crate,
# each in its own collapsible group in the GitHub Actions log. Every crate
# runs even after one fails, so one run reports all of them.
set -uo pipefail

# `tr -d '\r'`: jq.exe on a Windows runner ends its lines with CRLF, which
# would otherwise ride into every `-p` argument.
crates=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | tr -d '\r' | sort)
failed=()
# TEST_SKIP_CRATES: space-separated crates to leave out. The Windows CI job
# skips `knot`, whose GPUI window tests run past 30 minutes there.
for crate in $crates; do
    case " ${TEST_SKIP_CRATES:-} " in *" $crate "*) echo "skipping $crate"; continue ;; esac
    echo "::group::cargo test -p $crate"
    # TEST_CRATE_TIMEOUT (seconds; needs `timeout`, so not macOS): a hung crate fails by name
    # instead of holding the whole job until its timeout.
    if ! ${TEST_CRATE_TIMEOUT:+timeout "$TEST_CRATE_TIMEOUT"} cargo test -p "$crate"; then
        failed+=("$crate")
    fi
    echo "::endgroup::"
done

if [ ${#failed[@]} -gt 0 ]; then
    for crate in "${failed[@]}"; do
        echo "::error title=Tests failed::$crate"
    done
    echo "Tests failed in: ${failed[*]}"
    exit 1
fi
echo "Tests passed in every crate: $(echo $crates | tr '\n' ' ')"
