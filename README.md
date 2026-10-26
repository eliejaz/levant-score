# Levant Score · سهرة

A bilingual, mobile-first scorekeeper for Lebanese and Arab card nights. It ships with presets for Leekha (solo and teams), 400, Tarneeb, Chinese, Quatorze (14), Trex/Complex, Belote, Basra, and Hand. The custom preset covers any house game without changing code.

## Stack

- Rust REST API built with Axum and Tokio
- PostgreSQL with SQLx-native migrations and explicit transactional queries
- Stateless JWT authentication with BCrypt passwords
- Vite + React responsive frontend, English and Arabic RTL
- Built-in browser-only/PWA mode alongside account login
- Docker Compose for a one-command local setup

## Project structure

```text
backend/
├── game-rules/         Shared pure Rust scoring engine; native library + WebAssembly
├── src/
│   ├── api/             HTTP router, middleware, and API errors
│   ├── features/
│   │   ├── auth/        Authentication handlers and contracts
│   │   └── games/       HTTP handlers, repository queries, DTOs, and DB entities
│   └── infrastructure/  PostgreSQL connection and migrations
├── migrations/   Ordered SQLx database migrations
├── tests/        PostgreSQL API integration tests
├── Cargo.toml
└── Dockerfile
frontend/
├── public/       Static browser assets
├── src/
│   ├── components/
│   ├── lib/
│   └── pages/
├── package.json
└── Dockerfile
```

## Account and offline modes

One frontend deployment contains both modes. The home page explicitly offers sign-up, login, or offline play:

| Mode | Storage | Sign-in |
| --- | --- | --- |
| Account | Rust API + Neon PostgreSQL | Required |
| Offline | This browser's `localStorage` | None |

The offline edition keeps at most one active game and the latest ten finished games. When an eleventh game is finished, the oldest history item is removed automatically. It also registers a service worker, so after the first successful load it can open without a network connection. Browser data is device- and browser-profile-specific and can be removed by clearing site data.

`backend/game-rules` is the single authoritative implementation for presets, scoring, round validation, automatic game-ending rules, and structured winner outcomes. The API links it as a native Rust library. The frontend loads the compiled `frontend/public/game_rules.wasm`, so browser validation, offline scoring, and offline preset setup execute that exact same code rather than a JavaScript rewrite. JavaScript only adapts UI/storage data into the engine contract and renders localized helpers.

For local development, use the normal `npm run dev` command and choose either mode on the home page. Run `npm run test:local` to execute the browser-storage tests against the real WebAssembly engine.

When changing card-game rules, rebuild the tracked browser artifact before committing:

```bash
rustup target add wasm32-unknown-unknown
cd frontend
npm run rules:build
```

The normal Render frontend build does not require a Rust toolchain because the generated WebAssembly binary is committed with the source.

## Hosted deployment

The production app is split across three managed services:

| Part | Provider | Address |
| --- | --- | --- |
| React frontend | Render Static Site | <https://levant-score.onrender.com> |
| Rust API | Render Docker Web Service | <https://levant-score-back.onrender.com> |
| PostgreSQL database | Neon | Private connection; credentials are stored only in Render |

The browser loads the static React app from Render. Account mode calls the Rust API over HTTPS, and the API reads and writes data in Neon PostgreSQL. Offline mode never calls the API and keeps its limited game state on the device. The database is never contacted directly by the browser.

## Run with Docker

1. Copy `.env.example` to `.env` and replace `JWT_SECRET` and the database password.
2. From this directory, run:

   ```bash
   docker compose up --build
   ```

3. Open <http://localhost:3000>. The API is available at <http://localhost:27310/api>.

The default host ports deliberately avoid the commonly occupied `8080` and `5432` ports. You can change any of them in `.env`:

```dotenv
POSTGRES_PORT=5433
BACKEND_PORT=27310
FRONTEND_PORT=3000
```

PostgreSQL data is kept in the `levant_postgres` Docker volume.

## Run for development

Start PostgreSQL first, then:

```bash
cd backend
cargo run
```

In another terminal:

```bash
cd frontend
npm ci
npm run dev
```

The backend and frontend development defaults both use `http://localhost:27310`. Docker keeps the backend on port `8080` only inside its private network and publishes it on host port `27310`. You can override the frontend API with `VITE_API_URL`.

Run all API and shared-engine unit tests with `cargo test --workspace`. PostgreSQL integration tests additionally require a `DATABASE_URL` whose role may create temporary databases:

```bash
DATABASE_URL=postgresql://user:password@localhost/postgres cargo test --workspace --all-targets
```

## Included flows

- Create an account or sign back in
- Start a solo or team game from a preset
- Configure the target, score direction, seats, and teams
- Use rule-aware round forms with live helpers and server-side validation
- Keep Leekha team scores per player, with standard 101-point play or the shown-card double/triple variant to 201
- Count Chinese automatically from cards left, including its ×1/×2/×3/40 penalty ladder and 100-point loss rule
- Score Quatorze automatically from the winner, remaining-card values, Jokers, and normal or all-at-once finish
- Record 400 bids and made/missed results, Tarneeb contracts, and preset-specific card totals
- See live standings and score history
- Edit a saved hand; the app replays that hand and all later hands through the rules engine
- Undo the latest round
- Finish and reopen a game
- Start a zero-score rematch from a finished game while preserving its players and teams
- Create a fully custom game for any regional or house rules

See [RULES.md](RULES.md) for the scoring ownership and automatic end condition implemented for every preset.

## Deploy with Neon and Render

### 1. Create the Neon database

1. Create a project at [Neon](https://neon.com) and open **Connect**.
2. Copy the hostname, database name, username, and password. Prefer the pooled hostname for the deployed application.
3. Keep the credentials private. Do not commit the Neon connection string or password to Git.

The backend accepts a standard PostgreSQL URL through `DATABASE_URL`. The existing split variables are also supported:

```dotenv
DATABASE_URL=postgresql://<neon-role>:<url-encoded-password>@<neon-host>/<database>?sslmode=require
```

Or:

```dotenv
DB_URL=postgresql://<neon-host>/<database>?sslmode=require
DB_USER=<neon-role>
DB_PASSWORD=<neon-password>
```

SQLx migrations run automatically when the backend starts and create or update the schema in Neon.

> **Rust migration reset:** migration `0001_initial_schema.sql` intentionally drops the legacy Spring/Flyway application tables. The first deployment of this Rust version deletes existing users, games, players, rounds, and scores. Back up the Neon database first if you decide that data is needed.

> **Performance migration:** migration `0002_optimize_score_storage.sql` removes the unused score-entry UUID and compacts score storage. It clears existing games and score history once; user accounts are retained.

> **Cached-state migration:** migration `0003_cached_game_state.sql` clears existing games once, preserves user accounts, and adds transactionally maintained player totals and round counts. This keeps dashboard and score-entry queries constant-time as history grows.

> **Editable-round migration:** migration `0004_editable_rounds.sql` adds the original validated hand input to newly saved rounds. It does not delete existing data, but rounds created before this migration cannot be edited; they can still be viewed or undone.

> **Outcome migration:** migration `0005_game_outcomes.sql` stores the winner and end reason calculated by the shared rules engine and adds the dashboard's owner/status/date index. It preserves existing data; older finished games simply have no structured outcome.

### 2. Deploy the backend to Render

Create a **Web Service** from this repository with the following settings:

| Setting | Value |
| --- | --- |
| Runtime | Docker |
| Branch | `main` |
| Root directory | `backend` |
| Dockerfile | `./Dockerfile` |

Add these environment variables under the service's **Environment** page:

```dotenv
DATABASE_URL=postgresql://<neon-role>:<url-encoded-password>@<neon-host>/<database>?sslmode=require
JWT_SECRET=<a-random-secret-of-at-least-32-characters>
CORS_ORIGINS=https://levant-score.onrender.com
PORT=10000
DB_MAX_CONNECTIONS=5
```

`CORS_ORIGINS` must contain the frontend origin, without a trailing slash. If the frontend domain changes, update this value and redeploy the backend. Render supplies `PORT`; the Rust server binds to `0.0.0.0` on that port.

The current backend service is available at <https://levant-score-back.onrender.com>; its API routes are under `/api`.

### 3. Deploy the frontend to Render

Create a **Static Site** from the same repository:

| Setting | Value |
| --- | --- |
| Branch | `main` |
| Root directory | `frontend` |
| Build command | `npm install && npm run build` |
| Publish directory | `dist` |

Add this build-time environment variable:

```dotenv
VITE_API_URL=https://levant-score-back.onrender.com/api
```

Because this value is embedded in the Vite bundle during the build, changing it requires a new frontend build and deploy.

For React Router navigation and browser refreshes, add this rule in the Static Site's **Redirects/Rewrites** settings:

| Source | Destination | Action |
| --- | --- | --- |
| `/*` | `/index.html` | Rewrite |

The current frontend is available at <https://levant-score.onrender.com>.

### Updating production

Push changes to the deployed GitHub branch. With automatic deploys enabled, Render rebuilds the affected services. Otherwise, open each service and choose **Manual Deploy → Deploy latest commit**. Environment changes to `VITE_API_URL` require rebuilding the frontend; database and JWT changes require redeploying the backend.

On free plans, the backend or Neon compute may sleep while unused, so the first request after a quiet period can take longer.

### Performance notes

- Use Neon's pooled connection string (the hostname contains `-pooler`).
- Keep the Render backend and Neon database in the same region.
- The dashboard is paginated by the API, game history loads 20 hands at a time, and score mutations return only changed state.
- The API keeps no minimum idle database connections and defaults to five maximum connections, configurable with `DB_MAX_CONNECTIONS`.
- The frontend warms the API when account mode is selected and retries safe GET requests during a free-tier wake-up. Offline mode does not wait for the backend.

### Production security

- Use a unique, randomly generated `JWT_SECRET` of at least 32 characters.
- Never expose `DB_PASSWORD`, `DB_URL`, or `JWT_SECRET` to the Vite frontend.
- Keep `CORS_ORIGINS` restricted to the real frontend domain.
- Add rate limiting and email verification before opening public registration at scale.
