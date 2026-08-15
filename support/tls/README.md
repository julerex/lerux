# Smoke TLS credentials (Phase 51)

Test-only CA and server cert for `just test-fetch-tls`. **Not a production PKI.**

| File | Use |
|------|-----|
| `lerux-smoke-ca.der` | Baked into `tls-proxy` / `lerux-tls` as the sole trust anchor |
| `lerux-smoke-ca.pem` / `.key` | Re-issue the server cert |
| `lerux-smoke-server.pem` / `.key` | Host `lerux https-one` (SAN: `host`, `10.0.2.2`, `127.0.0.1`) |

Validity: 2026-08-15 → 2036-08-12. Guest rustls uses a fixed time in that window.

Regenerate:

```bash
# see the openssl invocations in the commit that added this directory
```
