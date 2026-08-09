# Theme Park Management

Multiplayer management game of theme parks.

## Prerequisites

- **Docker Desktop** (Windows/macOS, **WSL2** backend required on Windows) or **Docker Engine** (Linux) — everything else (Rust, Node, pnpm, protoc) is baked into the images, nothing to install by hand.
- **Windows only**: clone the repo **inside the WSL2 filesystem** (e.g. `~/code/ThemeParkManagement`), not under `/mnt/c/...` — otherwise bind mount file-watching becomes very slow (well-known Docker-on-Windows pitfall).

## Running the game

`cp .env.example .env`

`docker compose up`

That's it — a single command after copying `.env`. In order: Postgres starts and waits until ready, `pnpm install` runs once (shared between gateway/client), then the engine, gateway and client start together.

- Client: http://localhost:5173
- Gateway (health check): http://localhost:4000/health

The first run takes longer (image builds). Subsequent runs are fast (Docker cache + named volumes for `node_modules`/`target/`).

## Useful commands

Get a shell in an already-running container:

`docker compose exec engine bash`

`docker compose exec gateway sh`

`docker compose exec client sh`

Run a one-off command without a persistent container:

`docker compose run --rm engine cargo test` 

Follow a service's logs:

`docker compose logs -f engine`
