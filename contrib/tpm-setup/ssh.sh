#!/usr/bin/bash

set -Eeuo pipefail

pkcs11-tool \
    --module "${PKCS11_PROVIDER_MODULE}" \
    --keypairgen \
    --login --pin="${PKCS11_USER_PIN}" \
    --label="ssh" \
    --usage-sign \
    --key-type EC:prime256v1

echo "--- SSH Public Key ---"
ssh-keygen -D "${PKCS11_PROVIDER_MODULE}"
echo "--- SSH Public Key ---"
