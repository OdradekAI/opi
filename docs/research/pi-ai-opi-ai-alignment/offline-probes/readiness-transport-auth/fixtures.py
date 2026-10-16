"""Generate disposable synthetic RSA and TLS inputs; keep keys out of results."""
import datetime
import json
from pathlib import Path
import sys

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec, ed25519, rsa
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID

destination = Path(sys.argv[1]).resolve()
destination.mkdir(parents=True, exist_ok=True)
public = []
for bits, exponent in ((1024, 65537), (2048, 65537), (2048, 3), (4096, 65537), (8192, 65537)):
    key = rsa.generate_private_key(public_exponent=exponent, key_size=bits)
    for fmt, kind in ((serialization.PrivateFormat.PKCS8, "pkcs8"),
                      (serialization.PrivateFormat.TraditionalOpenSSL, "pkcs1")):
        name = f"rsa-{bits}-e{exponent}-{kind}"
        for encoding, ext in ((serialization.Encoding.PEM, "pem"),
                              (serialization.Encoding.DER, "der")):
            (destination / f"{name}.{ext}").write_bytes(key.private_bytes(
                encoding, fmt, serialization.NoEncryption()))
        public.append({"id": name, "bits": bits, "exponent": exponent, "format": kind})

now = datetime.datetime.now(datetime.timezone.utc)
ca_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
ca_name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, "opi disposable research CA")])
ca = (x509.CertificateBuilder().subject_name(ca_name).issuer_name(ca_name)
      .public_key(ca_key.public_key()).serial_number(x509.random_serial_number())
      .not_valid_before(now - datetime.timedelta(days=1))
      .not_valid_after(now + datetime.timedelta(days=1))
      .add_extension(x509.BasicConstraints(ca=True, path_length=None), critical=True)
      .sign(ca_key, hashes.SHA256()))
for encoding, ext in ((serialization.Encoding.PEM, "pem"), (serialization.Encoding.DER, "der")):
    (destination / f"ca.{ext}").write_bytes(ca.public_bytes(encoding))

for name, key, client, expired in (
    ("server", rsa.generate_private_key(public_exponent=65537, key_size=2048), False, False),
    ("expired", rsa.generate_private_key(public_exponent=65537, key_size=2048), False, True),
    ("client-rsa", rsa.generate_private_key(public_exponent=65537, key_size=2048), True, False),
    ("client-ec", ec.generate_private_key(ec.SECP256R1()), True, False),
    ("client-ed25519", ed25519.Ed25519PrivateKey.generate(), True, False),
):
    subject = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, name)])
    builder = (x509.CertificateBuilder().subject_name(subject).issuer_name(ca.subject)
               .public_key(key.public_key()).serial_number(x509.random_serial_number())
               .not_valid_before(now - datetime.timedelta(days=2 if expired else 1))
               .not_valid_after(now - datetime.timedelta(hours=1) if expired else now + datetime.timedelta(days=1))
               .add_extension(x509.BasicConstraints(ca=False, path_length=None), critical=True)
               .add_extension(x509.ExtendedKeyUsage([ExtendedKeyUsageOID.CLIENT_AUTH if client else ExtendedKeyUsageOID.SERVER_AUTH]), critical=False))
    if not client:
        builder = builder.add_extension(x509.SubjectAlternativeName([x509.DNSName("localhost")]), critical=False)
    cert = builder.sign(ca_key, hashes.SHA256())
    for encoding, ext in ((serialization.Encoding.PEM, "pem"), (serialization.Encoding.DER, "der")):
        (destination / f"{name}-cert.{ext}").write_bytes(cert.public_bytes(encoding))
        (destination / f"{name}-key.{ext}").write_bytes(key.private_bytes(
            encoding, serialization.PrivateFormat.PKCS8, serialization.NoEncryption()))
(destination / "rsa-cases.json").write_text(json.dumps(public, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"rsa_cases": len(public), "certificates": 6,
                  "private_keys": "disposable synthetic inputs; not preserved in repository"}))
