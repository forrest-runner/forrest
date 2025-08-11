#!/bin/bash

set -e -u -o pipefail

VERSION="<RUNNER_VERSION>"
HASH="<RUNNER_HASH>"

FILE="actions-runner-linux-x64-${VERSION}.tar.gz"
URL="https://github.com/actions/runner/releases/download/v${VERSION}/${FILE}"

if ! test -e "${FILE}"
then
    if test -x /usr/bin/apt-get
    then
        sudo DEBIAN_FRONTEND=noninteractive DPKG_FORCE=confnew apt-get update
        sudo DEBIAN_FRONTEND=noninteractive DPKG_FORCE=confnew apt-get \
            --assume-yes install icu-devtools
    fi

    curl --location --output "${FILE}" "${URL}"
    echo "${HASH} ${FILE}" > "${FILE}.hash"
    sha256sum --check "${FILE}.hash"

    mkdir --parents runner
    tar --extract --file "${FILE}" --directory runner
fi

export FORREST_API_URL="http://10.0.2.2:8080"
export FORREST_RUN_TOKEN_FILE="/home/runner/config/run-token"

./runner/run.sh --jitconfig <JITCONFIG>
