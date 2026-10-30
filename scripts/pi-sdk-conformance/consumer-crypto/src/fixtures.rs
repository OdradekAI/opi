//! Minimal synthetic loopback TLS corpus for the packaged crypto consumer.
//!
//! Same deterministic throwaway identities as the workspace tls_contract
//! tests, with matching long-lived certificates and keys (trusted CA, RSA-2048
//! server and client). Loopback-only; no real
//! credentials or endpoints.

pub const CA_PEM: &str = concat!(
    "-----BEGIN CERTIFICATE-----\n",
    "MIIDDTCCAfWgAwIBAgIUP1RAIngyR4XljxtNowWMR8sZXXAwDQYJKoZIhvcNAQEL\n",
    "BQAwDTELMAkGA1UEAwwCY2EwIBcNMjYxMDI5MDk0MDM0WhgPMjA1NjEwMjEwOTQw\n",
    "MzRaMA0xCzAJBgNVBAMMAmNhMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKC\n",
    "AQEAvUI5OGK0v70W3k2ErEqhp1Zy4WjK1mX2INtBVUjZXGWk4IosbZlO5rZITHRA\n",
    "CHSSx6Zd085kjJrAL9tFyPV7BBOu99HGYFTxxfQLohpdftHM6GpLlQFu32DI8XLw\n",
    "ErHvnsIPDGmf2E2Zw9g9fMYgAcXYTEkWfKe/pUrMNjabwidmbbcK9Z0RsUaI/0My\n",
    "gGcJ662JBxxiWlIbipl44H1l4Jnd3SZS1EFcXg0dNqoG438CWwEJfFn+T8MD3y33\n",
    "ALqPANte7Z6fPOKqnIdCdWcvqRKtdagW5DBJQ5kihgZQSZef+Gv8Ej1ijLQ1bM+q\n",
    "4rlaqH25Yiw5Hkto3dqggwehrQIDAQABo2MwYTAdBgNVHQ4EFgQUUg8eG7dOkyrR\n",
    "VB4gzZDinXeRwZ0wHwYDVR0jBBgwFoAUUg8eG7dOkyrRVB4gzZDinXeRwZ0wDwYD\n",
    "VR0TAQH/BAUwAwEB/zAOBgNVHQ8BAf8EBAMCAQYwDQYJKoZIhvcNAQELBQADggEB\n",
    "ACEp9XAImf53mOT7qb2WyPA6aPoJ+kPmZfLWaFBLjXgf3hdDJ4iF24Nt1hP9dJK+\n",
    "JHbEzXjNsItSEPjFqoJ8muw77BZu1Nbq/YIDVHwiAYM73Fit6pm/IHDhWEFBq7yz\n",
    "OLO82/IdpgcRNbyJSgUoWKwLrSdK0sLwrr0pRRqN7k6p/MoNYoVhI8yK2wx8f6kR\n",
    "HcqaePD5cw2ObppYUl42XN9J+0PA/ng/t28DzJ6hvjFXt2WyZEE4T0s/s+NmhB2r\n",
    "xWC7SHms5uHDtefwW9AG2iwPTwPoX+i9zOtGIwuIMQ/hUlj3THS0oN6yfnnIrmAa\n",
    "VQkRBDgqNVCBryuTyp9wrPA=\n",
    "-----END CERTIFICATE-----\n"
);

pub const SERVER_CERT_PEM: &str = concat!(
    "-----BEGIN CERTIFICATE-----\n",
    "MIIDPzCCAiegAwIBAgIUWTNvTJLSW58kpkTh1dGSmU3czjIwDQYJKoZIhvcNAQEL\n",
    "BQAwDTELMAkGA1UEAwwCY2EwIBcNMjYxMDI5MDk0MDM1WhgPMjA1NjEwMjEwOTQw\n",
    "MzVaMBUxEzARBgNVBAMMCnNlcnZlci1yc2EwggEiMA0GCSqGSIb3DQEBAQUAA4IB\n",
    "DwAwggEKAoIBAQCgcFffaWKKeYGHoPBmW1OKSOSDwiVToRXdxD4CEtPSEAt/MSbS\n",
    "GvJXSacNf4ouAYADRlhMMdopYrqjkARyZqSZl124wsFMX9GtmAozI7D5p8ljgOnP\n",
    "MkZ7SOc1ap9LfO2Bni9vNMA7Kt2nlDqTwE/be5tukX6T0x6JLKNus6pKQn2QxzVl\n",
    "41Q2PowacM2S+HQGTglO8ggoM7e2HhbOxKt0m9VVJRvjvJRKXGuj1T4C+UfGjd28\n",
    "8950ote7SwWSVa9EaYDj8dxXH+FDah194wsGWhTm3bd1CKAAqWKtdWSYHd4UmMLU\n",
    "cwoAIBXeQY1HrWS09P31z6wQrJOzdUTrofb3AgMBAAGjgYwwgYkwCQYDVR0TBAIw\n",
    "ADALBgNVHQ8EBAMCBaAwEwYDVR0lBAwwCgYIKwYBBQUHAwEwGgYDVR0RBBMwEYIJ\n",
    "bG9jYWxob3N0hwR/AAABMB0GA1UdDgQWBBSUAmsm1Q+mrcbr9GMOWsMra4123TAf\n",
    "BgNVHSMEGDAWgBRSDx4bt06TKtFUHiDNkOKdd5HBnTANBgkqhkiG9w0BAQsFAAOC\n",
    "AQEAtpGfJDuvmWW/r+lMh60MZyYJvnKmrBercEIfYXw9lS9KSpLgFnmrlQPtCOrR\n",
    "qMEJPkciExZJIDfKzY9eVwzfDw4o6XNfLEiKlCcxzm93MJKUOvzfinTsHmZ273Vs\n",
    "7Njm7p/5CG0YDyq1tc/6t8dfM1witPqZv89BFJKO9YFrp3KG3n12szvUS53v98jH\n",
    "Gj8JipX5X6Qr+e9+BWdylhPajVSfMC5yi3/4V4u/W70WvF1weZnYAWgdsiFyIQCz\n",
    "F1Ax3LH7UlzGm6IuWnOFBnDUepEh6fE/N3+s1IrNulTzqAesaYcwlofV+N317Uv5\n",
    "ggQ8AS3Mbd1ahcx1ye+T7tWrxQ==\n",
    "-----END CERTIFICATE-----\n"
);

pub const SERVER_KEY_PEM: &str = concat!(
    "-----BEGIN PRIVATE KEY-----\n",
    "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCgcFffaWKKeYGH\n",
    "oPBmW1OKSOSDwiVToRXdxD4CEtPSEAt/MSbSGvJXSacNf4ouAYADRlhMMdopYrqj\n",
    "kARyZqSZl124wsFMX9GtmAozI7D5p8ljgOnPMkZ7SOc1ap9LfO2Bni9vNMA7Kt2n\n",
    "lDqTwE/be5tukX6T0x6JLKNus6pKQn2QxzVl41Q2PowacM2S+HQGTglO8ggoM7e2\n",
    "HhbOxKt0m9VVJRvjvJRKXGuj1T4C+UfGjd288950ote7SwWSVa9EaYDj8dxXH+FD\n",
    "ah194wsGWhTm3bd1CKAAqWKtdWSYHd4UmMLUcwoAIBXeQY1HrWS09P31z6wQrJOz\n",
    "dUTrofb3AgMBAAECggEAAOyCOfPjF/5FR5mU4WvbDelCxmASv+ASXAJAg56K54+H\n",
    "yM1ou1t1Shk7jcx/mYIRAsxul+L16UJ7g2wQun10g6ciaGkwiP5cQkbgeCjDr6ni\n",
    "S8OebasqcfdL0ADujx/z9itFBWzqB7IdfsrtaZQWsGcr0ESQ+7igRXctG5hkVi5b\n",
    "CgWDuEXeAK3yio+8QFVnz+PtCKGXhURZRrIiSZtY6WR1OwsUhRgXnLqPbkmFfrzs\n",
    "NhfxTy7Jcnt0T5qr371PwWXOJtt/Kz0caAtVXlJd84mATqLfDYM7rscXJglUGF0b\n",
    "k1Rr4EkGNi248SWJiLZ7edIzFbPN8ieRJ1IFK3ztwQKBgQDSm+dl5xy/9r79WwB+\n",
    "b2MpK6FPvAH3DQZLUEv8fBgZnk3sRWJEYzFWhfE8ocEoeIPkmoPVA1GvmlrZPQEb\n",
    "+g/uNNeJ4IC0UeKv2OZvjoWH9pLV7zYHIVb+iZ+eHRhsQLw+IFl5C+D7iR1OzNkY\n",
    "KARojkMSYjHwLChrE+VhEn61UQKBgQDDBFyYHmqob79UYJhAEGeWXj6EmPvccmiK\n",
    "lI6/mXh0UpwShL+xtY/gOUqeaeYalTzOLyuFnRPSx4K70YeG5xizAz2HUGLx+92G\n",
    "awD/x62vewQx1DUZ/nHjuj6gANrYgtCaVbkzLevjYd6+O98SpSDE15mdlJQO42K1\n",
    "2WrScLV1xwKBgQCdqkUSzm+BbhEQzh2SiYSsf5Mlg9PcxQ1fOF+KRsQr+YzQQikM\n",
    "lU2MPn2YLGfUClSCwQHMZ82xLlawprC75voiE8D/4Oe5Fkwj5Lat8P4aCQdnzMqf\n",
    "j9/2ccTis1ljQRsrT0BsE+57n+wm38fuA5c5RrcDfitwZz5i2uQvRY9B4QKBgA/s\n",
    "QD8vZJgCH8Yc6PF6JWsSoNokv5O9R6ng/VLJjhs5T4L3kctJvsiZBO2wSdhrzP4s\n",
    "vcn4DFFOdu8s6XrrTYn4PsLYnmuRnAHIpNk9jYHhcnM7F2ba7qOgZlKcH0JEVBH1\n",
    "49w46KS1LgGsU8dq8iUAUYubQQeCK2IbsWwjRb2rAoGBAJoGF54/UBoCp/X8vByh\n",
    "DNkQ34vgDso6hzR4AHK2sppHHuUVB49rFfmT59BzneKVs9/RKnIge0BnmOGRTPyM\n",
    "2PPXYnPcscOUOv2Vyn4s7yKbU7FIZKKGDfNTDI0kzeMb7yGW875JuXjGBOTj12U5\n",
    "hNKdMrvXKPoI83rlmytZkSx2\n",
    "-----END PRIVATE KEY-----\n"
);

pub const CLIENT_CERT_PEM: &str = concat!(
    "-----BEGIN CERTIFICATE-----\n",
    "MIIDJTCCAg2gAwIBAgIUWTNvTJLSW58kpkTh1dGSmU3czjUwDQYJKoZIhvcNAQEL\n",
    "BQAwDTELMAkGA1UEAwwCY2EwIBcNMjYxMDI5MDk0MDM1WhgPMjA1NjEwMjEwOTQw\n",
    "MzVaMBkxFzAVBgNVBAMMDmNsaWVudC1yc2EyMDQ4MIIBIjANBgkqhkiG9w0BAQEF\n",
    "AAOCAQ8AMIIBCgKCAQEAriPcZncQ8LC6YmSORKtvtJVTawvLyF6f8EJGwe8qAr+e\n",
    "BSxgsK/bOkLMT7yDC7YxUg9dHOPQ99tKDoUPetGaObocnxSFtMBdLeYTjzumm2gv\n",
    "Dzpr2/R9gDaIv8nAa7fWlwThxRp2tcoUkHK5K/LF8Zxn2zLMgOKVzcckCaRBQXcA\n",
    "Bmi8I4E+yN5dkqdka/Nn2v1hVBIKeLnHUiSD5bw1nkyrPUMIzHNxaisMcLDd2QWr\n",
    "nB9zPxbWepWN8zYHVfkhPjW75SC8zT5QzsQPYajx9Y39IZXEHWbQ1temmv5XluEU\n",
    "KJ+8vCgtyDbj0J9ZGel1T/fhNWczvTa98LiJwmOXYwIDAQABo28wbTAJBgNVHRME\n",
    "AjAAMAsGA1UdDwQEAwIFoDATBgNVHSUEDDAKBggrBgEFBQcDAjAdBgNVHQ4EFgQU\n",
    "x51queHR5ZGrsVXQWuOZTmqedAgwHwYDVR0jBBgwFoAUUg8eG7dOkyrRVB4gzZDi\n",
    "nXeRwZ0wDQYJKoZIhvcNAQELBQADggEBABDFBduxmNsugSp/YRCZfO4oTvTU5k7Z\n",
    "e9LMPe7B3DqN0YYGFRfP7tJ4v0bHGL+3FsTuU723ydI1fHmVVm6qOGhoduRsrfvU\n",
    "e3ywRTGDwcQm+B792E2BKEmb/XpUaKAqrReFxiiQkGqFYWVxgrRz5yiy8Efchf6L\n",
    "B39cmnToGjx/h7UYeKHmsf9ghk2iam6efE5CR6pvhlHK8mJStdF5aZaBtDMnUjro\n",
    "FwUbYuMFoc4jB6Ni7flm0BevWz9hQnS2gx1nbfp5fjgLXo4OG+KZ5Io98fluLgQj\n",
    "5DJlbZ6zG6Zl6Aegw8TRsuGVRkOdxCYNEuGjcEcUXt+XG8yxBI1qJLg=\n",
    "-----END CERTIFICATE-----\n"
);

pub const CLIENT_KEY_PEM: &str = concat!(
    "-----BEGIN PRIVATE KEY-----\n",
    "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCuI9xmdxDwsLpi\n",
    "ZI5Eq2+0lVNrC8vIXp/wQkbB7yoCv54FLGCwr9s6QsxPvIMLtjFSD10c49D320oO\n",
    "hQ960Zo5uhyfFIW0wF0t5hOPO6abaC8POmvb9H2ANoi/ycBrt9aXBOHFGna1yhSQ\n",
    "crkr8sXxnGfbMsyA4pXNxyQJpEFBdwAGaLwjgT7I3l2Sp2Rr82fa/WFUEgp4ucdS\n",
    "JIPlvDWeTKs9QwjMc3FqKwxwsN3ZBaucH3M/FtZ6lY3zNgdV+SE+NbvlILzNPlDO\n",
    "xA9hqPH1jf0hlcQdZtDW16aa/leW4RQon7y8KC3INuPQn1kZ6XVP9+E1ZzO9Nr3w\n",
    "uInCY5djAgMBAAECggEAFzd1v315VzWgq9SPOS2IF5HfDMMGWMDw3uXfLCT4O2ST\n",
    "q0nm3Ded6U8VkT3nVMzmF3Qv7FWdOyZT4cqwrSW7w52CCl2AlAAfjCqEChEgXdDO\n",
    "kV+vGKSohMsAgV1jF5lrdTfz58Tm9caq5jU9ZdQYuuUo3YsQw9Axu70km1p4TsNY\n",
    "AuXUdft9TtbRAB9nbBD1CpdMafyaYA6pnH2D31WfjB58TGdBrQYvxj0vWASK2ao2\n",
    "QXQ1iCDD0YEkXBkxqpA553ojOe19xRiPmg0zK7mVxMPGDGrI2bIY9iw5BiwOATyF\n",
    "m52PKjNC0dUezPTknaCH2G+IONOeTkbvDxhshQLckQKBgQDUqYyGfYIB7eUjFleR\n",
    "MGDohKhP2j2hgKcve7Tvv3rc0lctnPmlCvKkBrsTfBLb5fd+fK0FGOVt3vlIApCL\n",
    "5rgI1rPR+mNYMsUfZDoRMEujK6byyRYPKmU4dCyhExiLJAITq0idx27r4MC+w3QX\n",
    "N8Kc6EJ9e4P0UGJHB01wp/HEsQKBgQDRoKIoYaiXkfEwyha8Fl22TP2YkLJ039dc\n",
    "OW73qvrXn/6tqkLpiqFQwK7B/9PRsBr3tLY2w6Ma3qXQYqlxxeUCcFNDhqmngU9+\n",
    "qODjIy0gMWok7c+6rYKyWd95xxfhTRMZn8bBrMv6mi/pP/t/dGS2AlKtNk+A8NRd\n",
    "47we/opyUwKBgQCOYkKzHT85WXVsK55PmGuZ4o/qfv16r2/Ai/wV9UcmqDI0WQCQ\n",
    "rybMCbGhlxXtYyh06LbTTdr22C5hN05eTQh06oOVGCcBu2Rd4zY5ofNSeXya22XI\n",
    "m6MPzEDBS87IAkDduHWzJf8vMNj8IFMO5IeJlLxH1bYAkv1p/wqrXday0QKBgAiZ\n",
    "361qfGFjspK+5/nlrQiq1gjpfvj+qjZ1gNEWqd4pCWkUwFQWxRGsJj99gLJs2F5r\n",
    "gsJyeoHCuuj2817gsMnN1AzEPLfLvXhD854+Nj7I6EMFou1U6zToLRwxCBDNecgm\n",
    "XyBZN7FbnDWk02viWbOCGuRSgyaWOxBzJKfc589TAoGBAJ7WEn8BFYrB4QGl30nP\n",
    "Z3bpq7YpDzQWQ8sHr9OpQIuNNKnOwf4a/oRlMBLzvKr0a0iGA1N7QhuVV8VExvzy\n",
    "dgjIXelKhYdaQRNM63KzozaGhn6R4hYaLhf6fYDzKgGFwcUsD46hQS/APwAHte2h\n",
    "CNu5Jucb9ZsbSPj9JJVs+EK3\n",
    "-----END PRIVATE KEY-----\n"
);

/// Admitted P-521 key (PKCS#8) used for the mismatched-identity negative.
pub const CLIENT_P521_KEY_PEM: &str = concat!(
    "-----BEGIN PRIVATE KEY-----\n",
    "MIHuAgEAMBAGByqGSM49AgEGBSuBBAAjBIHWMIHTAgEBBEIBfR8FPVUPgcJv4GsL\n",
    "yUTkHpIXPYFH82353wfKUo6I0dxhBDzYC2QGoLaA+Zp4KK009SNrXDAE4MpNSvZx\n",
    "JyjNe+OhgYkDgYYABAFlnO5a6BzGW3aV05OESZpv81sgPIahaZoPdyi7ZrC5MDHu\n",
    "O67EBXW84H/ITnB/V2rEY/DL3GTSgJVp2X4mc2JN4AAEQoVK/3ucCwxSeuNAVKbj\n",
    "x6q0Zp48h7l42tcqqzclDuzt757u/JB8JRTyQsPLPfEH9LXLvZiJ3mcsM3AjecpX\n",
    "Lw==\n",
    "-----END PRIVATE KEY-----\n"
);
