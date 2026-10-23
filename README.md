# Levant Score · سهرة

A bilingual, mobile-first scorekeeper for Lebanese and Arab card nights. It ships with presets for Leekha (solo and teams), 400, Tarneeb, Chinese, Trex/Complex, Belote, Basra, and Hand. The custom preset covers any house game without changing code.

## Stack

- Rust REST API built with Axum and Tokio
- PostgreSQL with SQLx-native migrations and explicit transactional queries
- Stateless JWT authentication with BCrypt passwords
- Vite + React responsive frontend, English and Arabic RTL
- Docker Compose for a one-command local setup

## Project structure

```text
backend/
├── src/
│   ├── api/             HTTP router, middleware, and API errors
│   ├── features/
│   │   ├── auth/        Authentication handlers and contracts
│   │   └── games/       Handlers, DTOs, domain rules, and DB entities
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

## Hosted deployment

The production app is split across three managed services:

| Part | Provider | Address |
| --- | --- | --- |
| React frontend | Render Static Site | <https://levant-score.onrender.com> |
| Rust API | Render Docker Web Service | <https://levant-score-back.onrender.com> |
| PostgreSQL database | Neon | Private connection; credentials are stored only in Render |

The browser loads the static React app from Render, the frontend calls the Rust API over HTTPS, and the API reads and writes data in Neon PostgreSQL. The database is never contacted directly by the browser.

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

Run the Rust unit tests with `cargo test`. PostgreSQL integration tests additionally require a `DATABASE_URL` whose role may create temporary databases:

```bash
DATABASE_URL=postgresql://user:password@localhost/postgres cargo test --all-targets
```

## Included flows

- Create an account or sign back in
- Start a solo or team game from a preset
- Configure the target, score direction, seats, and teams
- Use rule-aware round forms with live helpers and server-side validation
- Keep Leekha team scores per player and end when a player reaches 101
- Record 400 bids and made/missed results, Tarneeb contracts, and preset-specific card totals
- See live standings and score history
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

> **Performance migration:** migration `0002_optimize_score_storage.sql` removes the unused score-entry UUID, adds the index used by dashboard totals, and clears existing games and score history once. User accounts are retained.

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

### Production security

- Use a unique, randomly generated `JWT_SECRET` of at least 32 characters.
- Never expose `DB_PASSWORD`, `DB_URL`, or `JWT_SECRET` to the Vite frontend.
- Keep `CORS_ORIGINS` restricted to the real frontend domain.
- Add rate limiting and email verification before opening public registration at scale.
