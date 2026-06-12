#!/usr/bin/bash

set -Eeuo pipefail

# Print an error message if any command fails.
trap 'echo "Error: command failed at ${BASH_SOURCE[0]}:${LINENO}" >&2' ERR

usage() {
    echo "Usage: $(basename "$0") <base dir> <user/repo/machine> [scripts]" >&2
    echo >&2
    echo "Initialize a software TPM and set ip up for ssh / x509 certificate"
    echo >&2
    echo "  Example: $(basename "$0") env forrest-runner/test/build ssh.sh"

    exit 1
}

main() {
    if [[ $# -lt 2 ]]; then
        usage
    fi

    local self="$(realpath "${0}")"
    local selfdir="$(dirname "${self}")"

    local base_dir="$1"
    local triplet="$2"

    if [[ ! "$triplet" =~ ^[A-Za-z0-9_-]+/[A-Za-z0-9_-]+/[A-Za-z0-9_-]+$ ]]; then
        echo "Error: argument must be of the form 'user/repo/machine'" >&2
        exit 1
    fi

    local state_file="${base_dir}/machines/${triplet}.swtpm"
    local store_dir="${base_dir}/machines/${triplet}/tpm/tpm2_pkcs11"

    mkdir -p "${store_dir}"

    swtpm_setup --tpm2 --ecc --tpmstate "file://${state_file}"

    swtpm socket \
        --tpm2 \
        --tpmstate "backend-uri=file://${state_file}" \
        --server type=unixio,path=server.sock \
        --ctrl type=unixio,path=server.sock.ctrl \
        --daemon \
        --pid file=swtpm.pid \
        --flags startup-clear

    if [[ -e "/usr/lib/x86_64-linux-gnu/pkcs11/libtpm2_pkcs11.so" ]]; then
        export PKCS11_PROVIDER_MODULE="/usr/lib/x86_64-linux-gnu/pkcs11/libtpm2_pkcs11.so"
    else
        export PKCS11_PROVIDER_MODULE="/usr/lib/pkcs11/libtpm2_pkcs11.so"
    fi

    export PKCS11_SO_PIN="${PKCS11_SO_PIN:-0000}"
    export PKCS11_USER_PIN="${PKCS11_USER_PIN:-0000}"

    export TPM2TOOLS_TCTI="swtpm:path=$(readlink -f server.sock)"
    export TPM2_PKCS11_TCTI="${TPM2TOOLS_TCTI}"
    export TPM2_PKCS11_STORE="${store_dir}"

    pkcs11-tool \
        --module "${PKCS11_PROVIDER_MODULE}" \
        --slot-index=0 \
        --init-token \
        --label=tpm \
        --so-pin="${PKCS11_SO_PIN}"

    pkcs11-tool \
        --module "${PKCS11_PROVIDER_MODULE}" \
        --init-pin --login --so-pin="${PKCS11_SO_PIN}" \
        --slot-index=0 \
        --new-pin="${PKCS11_USER_PIN}"

    echo "-----------------"
    echo "Run setup scripts"
    echo "-----------------"

    for script in "${@:3}"; do
        PATH="${selfdir}:${PATH}" ${script}
    done

    kill "$(cat swtpm.pid)"
}

main $*
