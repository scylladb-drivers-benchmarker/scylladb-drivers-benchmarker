#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
case "${PHASE:?}" in
    run) exec ./regex "${STEP:?}" ;;
    prepare|teardown) exit 0 ;;
    *) exit 1 ;;
esac
