# newfm

> A music scrobbling service — a modern, self-hosted alternative to last.fm.

## Architecture

```
newfm/
├── crates/
│   ├── api/      ← Axum HTTP API (handlers, middleware, router)
│   ├── core/     ← Domain logic (validation, models, password hashing)
│   ├── db/       ← SQLx queries (repositories for all entities)
│   └── worker/   ← Background jobs (cleanup expired sessions & now_playing)
├── migrations/
│   └── 0001_initial.sql
└── .env.example
```

**Stack:** Rust · Axum 0.8 · SQLx 0.8 · PostgreSQL + TimescaleDB · Redis (fred)

---

## Prerequisites

- Rust (stable, 2024 edition)
- PostgreSQL with [TimescaleDB](https://docs.timescale.com/self-hosted/latest/install/) extension
- Redis

---

## Getting started

```bash
# 1. Copy and edit the env file
cp .env.example .env

# 2. Create the database
createdb newfm

# 3. Run the API (migrations apply automatically on startup)
cargo run -p api

# 4. (Optional) Run the background worker
cargo run -p worker
```

---

## Environment Variables

| Variable             | Required | Default                  | Description                         |
|----------------------|----------|--------------------------|-------------------------------------|
| `DATABASE_URL`       | ✓        | —                        | PostgreSQL connection string        |
| `REDIS_URL`          | —        | `redis://127.0.0.1:6379` | Redis connection string             |
| `BIND_ADDR`          | —        | `0.0.0.0:8080`           | API listen address                  |
| `RUST_LOG`           | —        | —                        | Tracing filter (e.g. `newfm=debug`) |
| `DB_MAX_CONNECTIONS` | —        | `20`                     | Postgres pool size                  |
