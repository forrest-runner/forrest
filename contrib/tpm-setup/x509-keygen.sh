#!/usr/bin/bash

set -Eeuo pipefail

label="${X509_LABEL:-key}"
subject="${X509_LABEL:-/CN=${label}}"
cert_path="${TPM2_PKCS11_STORE}/../${label}.cert.pem"
key_path="${TPM2_PKCS11_STORE}/../${label}.key.pem"
key_uri="pkcs11:object=${label};pin-value=${PKCS11_USER_PIN}"

pkcs11-tool \
    --module "${PKCS11_PROVIDER_MODULE}" \
    --keypairgen \
    --login --pin "${PKCS11_USER_PIN}" \
    --label="${label}" \
    --usage-sign \
    --key-type EC:prime256v1

# Generate self-signed certificate using the keypair generated above

openssl req \
    -new -x509 \
    -provider default -provider base -provider pkcs11 \
    -days 36500 \
    -subj "${subject}" \
    -key "${key_uri}" \
    -out "${cert_path}"

pkcs11-tool \
    --module "${PKCS11_PROVIDER_MODULE}" \
    --login --pin "${PKCS11_USER_PIN}" \
    --label="${label}" \
    --write-object ${cert_path} \
    --type cert

tmp_dir="$(mktemp --directory)"

cat > "${tmp_dir}/asn1.conf" <<EOF
asn1=SEQUENCE:pkcs11_uri_seq

[pkcs11_uri_seq]
version=VISIBLESTRING:PKCS\#11 Provider URI v1.0
uri=UTF8:${key_uri}
EOF

openssl asn1parse \
    -genconf "${tmp_dir}/asn1.conf" \
    -noout \
    -out "${tmp_dir}/key.der"

{
    echo "-----BEGIN PKCS#11 PROVIDER URI-----"
    openssl base64 -in "${tmp_dir}/key.der"
    echo "-----END PKCS#11 PROVIDER URI-----"
} > "${key_path}"

rm -rf "${tmp_dir}"
