"""Additional disposable mTLS algorithms; independent synthetic trust root."""
import datetime
from pathlib import Path
import sys
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec, rsa
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID

p=Path(sys.argv[1]);p.mkdir(parents=True,exist_ok=True)
now=datetime.datetime.now(datetime.timezone.utc)
ca_key=rsa.generate_private_key(public_exponent=65537,key_size=2048)
ca_name=x509.Name([x509.NameAttribute(NameOID.COMMON_NAME,'synthetic supplement CA')])
ca=(x509.CertificateBuilder().subject_name(ca_name).issuer_name(ca_name).public_key(ca_key.public_key())
    .serial_number(x509.random_serial_number()).not_valid_before(now-datetime.timedelta(days=1))
    .not_valid_after(now+datetime.timedelta(days=1)).add_extension(x509.BasicConstraints(ca=True,path_length=None),True)
    .sign(ca_key,hashes.SHA256()))
for encoding,ext in ((serialization.Encoding.PEM,'pem'),(serialization.Encoding.DER,'der')):
 (p/f'ca.{ext}').write_bytes(ca.public_bytes(encoding))
for name,key,client in (
 ('server',rsa.generate_private_key(public_exponent=65537,key_size=2048),False),
 ('client-p521',ec.generate_private_key(ec.SECP521R1()),True),
 ('client-rsa-e3',rsa.generate_private_key(public_exponent=3,key_size=2048),True),
 ('client-rsa1024',rsa.generate_private_key(public_exponent=65537,key_size=1024),True)):
 subject=x509.Name([x509.NameAttribute(NameOID.COMMON_NAME,name)])
 builder=(x509.CertificateBuilder().subject_name(subject).issuer_name(ca_name).public_key(key.public_key())
          .serial_number(x509.random_serial_number()).not_valid_before(now-datetime.timedelta(days=1))
          .not_valid_after(now+datetime.timedelta(days=1))
          .add_extension(x509.ExtendedKeyUsage([ExtendedKeyUsageOID.CLIENT_AUTH if client else ExtendedKeyUsageOID.SERVER_AUTH]),False))
 if not client:builder=builder.add_extension(x509.SubjectAlternativeName([x509.DNSName('localhost')]),False)
 cert=builder.sign(ca_key,hashes.SHA256())
 for encoding,ext in ((serialization.Encoding.PEM,'pem'),(serialization.Encoding.DER,'der')):
  (p/f'{name}-cert.{ext}').write_bytes(cert.public_bytes(encoding))
  (p/f'{name}-key.{ext}').write_bytes(key.private_bytes(encoding,serialization.PrivateFormat.PKCS8,serialization.NoEncryption()))
print('synthetic supplement generated; no private key preserved in repository')
