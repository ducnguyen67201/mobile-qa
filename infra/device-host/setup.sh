#!/usr/bin/env bash
# Run explicitly as the dedicated device user. No cloud API, sudo, model or checks.
set -euo pipefail
if [[ $# != 1 ]]; then
  echo 'usage: setup.sh ABSOLUTE_DEDICATED_SDK_ROOT' >&2
  exit 2
fi
if [[ "$1" != /* || "$1" == / ]]; then exit 2; fi
python3 "$(dirname "$0")/install_tools.py" --sdk-root "$1"
echo 'Tools installed. Review Android SDK licenses and explicitly accept before use.'
echo 'Then configure profile.toml with absolute paths and run device-doctor.'
