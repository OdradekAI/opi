"""Memory-only private OpenSSL 3 libctx experiment; no Rust implementation.

Run in a fresh child with an intentionally poisoned OPENSSL_CONF. No global
initialization/options/default-provider setters or library config loader are
called. A deliberate default-context fetch AFTER private signing is a negative
control showing that ambient config is still effective for the host's context.
"""
import ctypes as C
import hashlib
import json
from pathlib import Path
import sys

dll_path, npm, fixture, node = map(Path, sys.argv[1:5])
library = C.CDLL(str(dll_path))
P, I, Z, B = C.c_void_p, C.c_int, C.c_size_t, C.c_char_p


def api(name, result, *args):
    function = getattr(library, name)
    function.restype = result
    function.argtypes = list(args)
    return function


version = api("OpenSSL_version", B, I)
new_context = api("OSSL_LIB_CTX_new", P)
free_context = api("OSSL_LIB_CTX_free", None, P)
provider_load = api("OSSL_PROVIDER_load", P, P, B)
provider_unload = api("OSSL_PROVIDER_unload", I, P)
decoder_new = api("OSSL_DECODER_CTX_new_for_pkey", P, C.POINTER(P), B, B, B, I, P, B)
decoder_free = api("OSSL_DECODER_CTX_free", None, P)
PASS = C.CFUNCTYPE(I, P, Z, C.POINTER(Z), P, P)
decoder_passphrase = api("OSSL_DECODER_CTX_set_passphrase_cb", I, P, PASS, P)
decode = api("OSSL_DECODER_from_data", I, P, C.POINTER(P), C.POINTER(Z))
key_free = api("EVP_PKEY_free", None, P)
md_new = api("EVP_MD_CTX_new", P)
md_free = api("EVP_MD_CTX_free", None, P)
sign_init = api("EVP_DigestSignInit_ex", I, P, C.POINTER(P), B, P, B, P, P)
padding = api("EVP_PKEY_CTX_set_rsa_padding", I, P, I)
sign_update = api("EVP_DigestSignUpdate", I, P, P, Z)
sign_final = api("EVP_DigestSignFinal", I, P, P, C.POINTER(Z))
md_fetch = api("EVP_MD_fetch", P, P, B, B)
md_release = api("EVP_MD_free", None, P)
peek_error = api("ERR_peek_error", C.c_ulong)
clear_error = api("ERR_clear_error", None)


@PASS
def reject_passphrase(_buffer, _size, _length, _params, _userdata):
    return 0


def sign(context, key_path, secured_input):
    key, decoder, md = P(), None, None
    clear_error()
    try:
        decoder = decoder_new(C.byref(key), b"PEM", None, b"RSA", 3, context, b"provider=default")
        if not decoder or decoder_passphrase(decoder, reject_passphrase, None) != 1:
            return {"accepted": False, "stage": "decoder-construction"}
        data = key_path.read_bytes()
        backing = C.create_string_buffer(data)
        cursor, length = P(C.addressof(backing)), Z(len(data))
        if decode(decoder, C.byref(cursor), C.byref(length)) != 1 or not key.value:
            return {"accepted": False, "stage": "decode"}
        md, pkey_ctx = md_new(), P()
        if sign_init(md, C.byref(pkey_ctx), b"SHA256", context, b"provider=default", key, None) != 1:
            return {"accepted": False, "stage": "sign-init"}
        if padding(pkey_ctx, 1) != 1:
            return {"accepted": False, "stage": "padding"}
        message = secured_input.encode("ascii")
        if sign_update(md, C.c_char_p(message), len(message)) != 1:
            return {"accepted": False, "stage": "update"}
        size = Z()
        if sign_final(md, None, C.byref(size)) != 1:
            return {"accepted": False, "stage": "length"}
        output = C.create_string_buffer(size.value)
        if sign_final(md, output, C.byref(size)) != 1:
            return {"accepted": False, "stage": "sign"}
        signature = output.raw[:size.value]
        return {"accepted": True, "signature_bytes": len(signature),
                "signature_sha256": hashlib.sha256(signature).hexdigest(),
                "error_queue_after": peek_error()}
    finally:
        if md:
            md_free(md)
        if decoder:
            decoder_free(decoder)
        if key:
            key_free(key)


import os
import subprocess

context = new_context()
assert context
provider = provider_load(context, b"default")
assert provider
cases = json.loads((fixture / "rsa-cases.json").read_text())
if (fixture / "rsa-multiprime.pem").exists():
    cases.append({"id": "rsa-multiprime", "bits": 2048, "primes": 3, "format": "pkcs8"})
observations = []
asset = Path(__file__).resolve().parent
for case in cases:
    command = [str(node), str(asset / "openssl-reference.cjs"), str(npm), str(fixture / (case["id"] + ".pem"))]
    # Node's own runtime is a distinct library; it must not receive the native
    # library's deliberately poisoned config as an accidental fixture policy.
    env = {k: os.environ[k] for k in ("SystemRoot", "PATH", "TEMP", "TMP") if k in os.environ}
    reference_process = subprocess.run(command, capture_output=True, timeout=15, env=env)
    assert reference_process.returncode == 0, reference_process.stderr
    reference = json.loads(reference_process.stdout)
    assert reference["accepted"] and reference["verified"]
    result = sign(context, fixture / (case["id"] + ".pem"), reference["secured_input"])
    assert result["accepted"] and result["signature_sha256"] == reference["signature_sha256"]
    observations.append({"kind": "rsa-private-libctx", **case,
                         "reference": reference, "native": result,
                         "reference_exit": reference_process.returncode})
for name, filename in (("malformed", "malformed.pem"),
                       ("encrypted-no-passphrase", "encrypted.pem"),
                       ("non-rsa", "client-ec-key.pem")):
    if not (fixture / filename).exists():
        continue
    command = [str(node), str(asset / "openssl-reference.cjs"), str(npm), str(fixture / filename)]
    result = subprocess.run(command, capture_output=True, timeout=15, env=env)
    assert result.returncode == 0
    reference = json.loads(result.stdout)
    rejection = sign(context, fixture / filename, observations[0]["reference"]["secured_input"])
    assert rejection["accepted"] is False
    if name != "non-rsa":
        assert reference["accepted"] is False
    observations.append({"kind": "rsa-private-libctx-negative", "id": name,
                         "reference": reference, "native": rejection})
negative = md_fetch(None, b"SHA256", None)
if negative:
    md_release(negative)
assert not negative, "Poisoned default context must reject SHA256; no global NO_LOAD_CONFIG allowed"
clear_error()
case = observations[0]
after = sign(context, fixture / (case["id"] + ".pem"), case["reference"]["secured_input"])
assert after["accepted"] and after["signature_sha256"] == case["reference"]["signature_sha256"]
observations.append({"kind": "isolation-negative-control", "ambient_default_sha256_fetch": False,
                     "private_sign_after_ambient_failure": after,
                     "global_no_load_config_called": False, "global_context_replaced": False})
provider_unload(provider)
free_context(context)
print(json.dumps({"library": str(dll_path), "version": version(0).decode(),
                  "library_sha256": hashlib.sha256(dll_path.read_bytes()).hexdigest(),
                  "observations": observations}, indent=2))
