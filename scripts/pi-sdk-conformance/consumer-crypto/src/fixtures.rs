//! Minimal synthetic loopback TLS corpus for the packaged crypto consumer.
//!
//! Same deterministic throwaway identities as the workspace tls_contract
//! tests (trusted CA, RSA-2048 server and client). Loopback-only; no real
//! credentials or endpoints.

pub const CA_PEM: &str = concat!(
    "-----BEGIN CERTIFICATE-----\n",
    "MIICzjCCAbagAwIBAgIUV6uosZqIBmwVe4FFVHtWnrvtrSkwDQYJKoZIhvcNAQEL\n",
    "BQAwDTELMAkGA1UEAwwCY2EwHhcNMjYxMDE4MDkyMzA3WhcNMjYxMTE4MDkyMzA3\n",
    "WjANMQswCQYDVQQDDAJjYTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEB\n",
    "AKNyRB4+9I4I4Nvmp1dK99Res7z/EqYFYjTkZ5BbK+Y+4OnsDT/mUasSDqn4DLn9\n",
    "83CXHfs6FLalMv4NvoBg8DbfMtfP0dPSBqdVjgRN6HOU9nXCn/KzjpROT0fj7f8w\n",
    "1eadziIu3IKlL6NBBDUHrsMPc36fZyYJ6YLVwuhswd8/Jk972ykMd3fIvzowFcKt\n",
    "bOJ8OH/irm0rH5YH2D/NzMSSADOQxZ+zsc19o0ZeL8G9q9d7hz1rFeZAiFfY7ml/\n",
    "wjpSEMO0mN50+g1wbhQl5ZjdEV/jTkzI6SYQNDNPz3JKvEBEd7kRbsHMn9xYw0w3\n",
    "MderNVhA4kzcbdirkgV00M0CAwEAAaMmMCQwEgYDVR0TAQH/BAgwBgEB/wIBADAO\n",
    "BgNVHQ8BAf8EBAMCAYYwDQYJKoZIhvcNAQELBQADggEBACayLtlSOPvyfhNnJDCM\n",
    "LBW4uNsvSwlcYQAd7eKoStaXmu2HocMq14kULgTSWYxwvgHjsPush8N0gR22A3Nc\n",
    "8zECXzY7Pwcce0jR0gs9aytdZcChtw3U7/4AFV/aS2N9eamrpvGwGbbjhkcfFL9Y\n",
    "6Ph1Y1qrCUV5royWUxJIuXjkvPUIBabg5kdJaXih6auzmifMjP47hKs3mtx/vI1u\n",
    "m+0Z7ild0ZCUaHSS/uMI21fMP1/dMC4/CJfviwASlCcDr11eSK2H9AfKtnDh9ZiJ\n",
    "q5fvmyjSWUicVgHJuHy9EQx5waceFKJH7PwO+boB9C9Mxl7NodXn98adI5c4MsFn\n",
    "McU=\n",
    "-----END CERTIFICATE-----\n"
);

pub const SERVER_CERT_PEM: &str = concat!(
    "-----BEGIN CERTIFICATE-----\n",
    "MIIDATCCAemgAwIBAgIUSJUFdeyMvOkGyjuSmtK3aYmRcEEwDQYJKoZIhvcNAQEL\n",
    "BQAwDTELMAkGA1UEAwwCY2EwHhcNMjYxMDE5MDgyMzA3WhcNMjYxMDI5MDkyMzA3\n",
    "WjAVMRMwEQYDVQQDDApzZXJ2ZXItcnNhMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8A\n",
    "MIIBCgKCAQEAoHBX32liinmBh6DwZltTikjkg8IlU6EV3cQ+AhLT0hALfzEm0hry\n",
    "V0mnDX+KLgGAA0ZYTDHaKWK6o5AEcmakmZdduMLBTF/RrZgKMyOw+afJY4DpzzJG\n",
    "e0jnNWqfS3ztgZ4vbzTAOyrdp5Q6k8BP23ubbpF+k9MeiSyjbrOqSkJ9kMc1ZeNU\n",
    "Nj6MGnDNkvh0Bk4JTvIIKDO3th4WzsSrdJvVVSUb47yUSlxro9U+AvlHxo3dvPPe\n",
    "dKLXu0sFklWvRGmA4/HcVx/hQ2odfeMLBloU5t23dQigAKlirXVkmB3eFJjC1HMK\n",
    "ACAV3kGNR61ktPT99c+sEKyTs3VE66H29wIDAQABo1EwTzAMBgNVHRMBAf8EAjAA\n",
    "MA4GA1UdDwEB/wQEAwIFoDATBgNVHSUEDDAKBggrBgEFBQcDATAaBgNVHREEEzAR\n",
    "gglsb2NhbGhvc3SHBH8AAAEwDQYJKoZIhvcNAQELBQADggEBAI/zoKE/ofTaGHZr\n",
    "yB5PE9PGjG5k1kKy1+u5ua/tJ3aMh5vRXU+Pp1/Zgb0uoN2tcUyQJuqik+jboxTT\n",
    "IlRK9JZDhK0v/wwzkk6DrW87IpLv8PCf4FhPrtbh+fetCzS2UNKTzbmtOOzk97nR\n",
    "XGoPF+3IOB2YaZceuoUKIjpPi3pTaPFIsFHi7sYljpd19MMOoPL5ZIFu8r7r0dLi\n",
    "tvGdB/Yf4LEFrP2mrH28HYBVROI17zUMryg+9u1pGw5e0Ip9LrhLPszCQjg3YiEy\n",
    "O62GlFIdYTQgLg3UFb4EeQXOkLpguJHwlPgjHvC9O3AvO/wuk1RJw0Nfg0WmetDS\n",
    "Qg3vOyQ=\n",
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
    "MIIC6TCCAdGgAwIBAgIUM/x5fyho0j/59Jt/jU3Oe36bdLcwDQYJKoZIhvcNAQEL\n",
    "BQAwDTELMAkGA1UEAwwCY2EwHhcNMjYxMDE5MDgyMzA3WhcNMjYxMDI5MDkyMzA3\n",
    "WjAZMRcwFQYDVQQDDA5jbGllbnQtcnNhMjA0ODCCASIwDQYJKoZIhvcNAQEBBQAD\n",
    "ggEPADCCAQoCggEBAK4j3GZ3EPCwumJkjkSrb7SVU2sLy8hen/BCRsHvKgK/ngUs\n",
    "YLCv2zpCzE+8gwu2MVIPXRzj0PfbSg6FD3rRmjm6HJ8UhbTAXS3mE487pptoLw86\n",
    "a9v0fYA2iL/JwGu31pcE4cUadrXKFJByuSvyxfGcZ9syzIDilc3HJAmkQUF3AAZo\n",
    "vCOBPsjeXZKnZGvzZ9r9YVQSCni5x1Ikg+W8NZ5Mqz1DCMxzcWorDHCw3dkFq5wf\n",
    "cz8W1nqVjfM2B1X5IT41u+UgvM0+UM7ED2Go8fWN/SGVxB1m0NbXppr+V5bhFCif\n",
    "vLwoLcg249CfWRnpdU/34TVnM702vfC4icJjl2MCAwEAAaM1MDMwDAYDVR0TAQH/\n",
    "BAIwADAOBgNVHQ8BAf8EBAMCBaAwEwYDVR0lBAwwCgYIKwYBBQUHAwIwDQYJKoZI\n",
    "hvcNAQELBQADggEBADO0U04qVXmw3fTveFBoJ7UXu7drdh4aq3FsOQo4bUcQrbtL\n",
    "hSX0/S8wnjFQnUHCUgTzsoileZpz0NG2A5std4BMCOG2E96ypO5Hw+1j+a4w5Csc\n",
    "SNm75RFoq9SdhHReAVeM+f0lZRHSx3oYl0ArPiwiMXMgaGPfumOg+qeRtjj4+A+v\n",
    "TCCEklZq8RBgd66jD30W79mHBqFHGN0ZYm8qCNCJO569yiRG5rDsbFa3YrFhoBlO\n",
    "LUhV4jk3oBuLQZL/4OPtuAQRS/7F4haUqC36/LJehLArrzcRHUA7SEVqH9JEqvx4\n",
    "YAOY/f/csQ1GReYMqqOPlEckYaptBWTZiGdUqy0=\n",
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
