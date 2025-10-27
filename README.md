# surevoucher_webcore

Minimal, opinionated HTTP/TLS server using Axum + Hyper (+ optional Rustls).
No Tower/CORS. Graceful shutdown. Integrated with `surevoucher_configcore`.

## Ports
- **App server**: binds to `SUREVOUCHER__HOST` / `SUREVOUCHER__PORT` (via ConfigCore).
- **Health server**: always runs on a **separate port**, default `127.0.0.1:18080`.
  - Override with `SUREVOUCHER__HEALTH_HOST` and `SUREVOUCHER__HEALTH_PORT`.
  - Exposes `GET /healthz` on the health port only (keep it off ingress).

## Run (HTTP)
```bash
cargo run
# app:    http://127.0.0.1:8080/
# health: http://127.0.0.1:18080/healthz
```

## Run (TLS)
```bash
cargo run --features tls
```

Ensure `../configcore` is checked out alongside this repo, or change the dependency in `Cargo.toml`.