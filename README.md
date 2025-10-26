# surevoucher_webcore

Minimal, opinionated HTTP/TLS server using Axum + Hyper (+ optional Rustls).
No Tower/CORS. Graceful shutdown. Integrated with `surevoucher_configcore`.

## Run (HTTP)
```bash
cargo run
```

## Run (TLS)
```bash
cargo run --features tls
```

Ensure `../configcore` is checked out alongside this repo, or change the dependency in `Cargo.toml`.