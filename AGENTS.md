# OpenNote — Project Overview

A Tauri desktop app with a React + TypeScript + Tailwind CSS frontend, following bulletproof-react architecture.

## Tech Stack

- **Frontend:** React 19, TypeScript, Vite, Tailwind CSS v4, shadcn/ui (new-york style)
- **Desktop:** Tauri v2 (Rust backend)
- **Routing:** react-router v7
- **Validation:** Zod (env vars, forms)
- **Linting:** Biome via ultracite, Husky + lint-staged for pre-commit hooks
- **Package manager:** bun

## Project Structure

```
opennote-rs/
├── src/                          # Frontend source
│   ├── app/                      # Application layer
│   │   ├── index.tsx             # Root component (AppProvider → AppRouter)
│   │   ├── provider.tsx          # Global providers (ErrorBoundary, TooltipProvider, Suspense)
│   │   ├── router.tsx            # Route definitions (react-router BrowserRouter)
│   │   ├── routes/               # Page-level route components
│   │   │   ├── home.tsx
│   │   │   └── not-found.tsx
│   │   └── global.css            # Tailwind base styles + shadcn CSS variables
│   ├── components/               # Shared UI components
│   │   └── ui/                   # shadcn/ui primitives (button, tooltip, etc.)
│   ├── config/
│   │   └── env.ts                # Zod-validated env vars (VITE_ prefix)
│   ├── features/                 # Feature-based modules
│   │   ├── built-with/           # "Built with" showcase (SVGs + component)
│   │   ├── errors/               # Error boundary pages (AppErrorPage, ErrorBase)
│   │   └── github-star-button/   # GitHub star button feature
│   ├── hooks/                    # Shared custom hooks (empty, add here)
│   ├── lib/
│   │   ├── create-env.ts         # Env validation helper (strips VITE_ prefix)
│   │   └── utils.ts              # cn() utility (clsx + tailwind-merge)
│   ├── stores/                   # Global state stores (empty, add here)
│   ├── types/                    # Shared TypeScript types (empty, add here)
│   ├── utils/                    # Shared utility functions (empty, add here)
│   ├── main.tsx                  # Entry point (ReactDOM render)
│   └── vite-env.d.ts             # Vite type declarations
├── src-tauri/                    # Rust backend (Tauri v2)
│   ├── src/main.rs               # Tauri app entry, command handlers (greet)
│   ├── Cargo.toml                # Rust dependencies
│   ├── tauri.conf.json           # Tauri config (window, bundling, plugins)
│   ├── capabilities/             # Tauri permission capabilities
│   ├── build.rs                  # Tauri build script
│   └── icons/                    # App icons for all platforms
├── public/                       # Static assets served as-is
├── components.json               # shadcn/ui config (aliases, style, icons)
├── biome.jsonc                   # Biome lint/format config (via ultracite)
├── vite.config.ts                # Vite config (React plugin, Tailwind, @ alias)
├── tsconfig.json                 # TypeScript config
└── package.json                  # Scripts, dependencies, lint-staged
```

## Key Conventions

- **Path alias:** `@/` maps to `src/` (configured in vite.config.ts and tsconfig.json)
- **Feature modules:** Each feature in `src/features/<name>/` can contain `api/`, `assets/`, `components/`, `hooks/`, `stores/`, `types/`, `utils/` subdirectories
- **shadcn/ui components:** Added via `npx shadcn@latest add <component>`, stored in `src/components/ui/`
- **Env vars:** Define with `VITE_` prefix in `.env`, validate with Zod schema in `src/config/env.ts`
- **Tauri commands:** Define in `src-tauri/src/main.rs`, register with `tauri::generate_handler![]`
- **Tauri plugins:** Registered in both `main.rs` (`.plugin()`) and `tauri.conf.json` (`plugins` key)

## Common Commands

| Command | Description |
|---------|-------------|
| `bun run dev` | Start Vite dev server (port 1420) |
| `bun run tauri dev` | Start Tauri dev (Rust + frontend) |
| `bun run build` | TypeScript check + Vite build |
| `bun run tauri build` | Production Tauri build |
| `bun run check` | Biome lint/format check |
| `bun run fix` | Biome lint/format auto-fix |
| `bun run typecheck` | TypeScript type check only |

## Architecture Notes

- **App bootstrap:** `main.tsx` → `App` (`app/index.tsx`) → `AppProvider` wraps `AppRouter`
- **Routing:** react-router `createBrowserRouter` with lazy-loaded route components
- **Error handling:** `react-error-boundary` at the provider level, with feature-scoped error pages in `features/errors/`
- **Styling:** Tailwind CSS v4 (Vite plugin), shadcn/ui new-york style, CSS variables for theming
- **Pre-commit:** Husky runs `lint-staged` → `ultracite fix` on staged files

## Agent skills

### Issue tracker

Issues are tracked in GitHub for `ca-lee-b/opennote`. See `docs/agents/issue-tracker.md`.

### Triage labels

The default five-label triage vocabulary is used as-is. See `docs/agents/triage-labels.md`.

### Domain docs

This is a single-context repo: root `CONTEXT.md` plus `docs/adr/` when present. See `docs/agents/domain.md`.
