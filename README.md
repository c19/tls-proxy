

## self signed cert
> https://devopscube.com/create-self-signed-certificates-openssl/

```bash
openssl req -x509 \
            -sha256 -days 356 \
            -nodes \
            -newkey rsa:2048 \
            -subj "/CN=demo.self-signed.com/C=US/L=San Fransisco" \
            -keyout rootCA.key -out rootCA.crt

openssl genrsa -out server.key 2048

cat > csr.conf <<EOF
[ req ]
default_bits = 2048
prompt = no
default_md = sha256
req_extensions = req_ext
distinguished_name = dn

[ dn ]
C = US
ST = California
L = San Fransisco
O = MLopsHub
OU = MlopsHub Dev
CN = demo.tls-proxy.com

[ req_ext ]
subjectAltName = @alt_names

[ alt_names ]
DNS.1 = demo.tls-proxy.com
DNS.2 = *.tls-proxy.com
IP.1 = 127.0.0.1

EOF

cat > cert.conf <<EOF

authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = demo.tls-proxy.com

EOF

openssl x509 -req \
    -in server.csr \
    -CA rootCA.crt -CAkey rootCA.key \
    -CAcreateserial -out server.crt \
    -days 365 \
    -sha256 -extfile cert.conf
```