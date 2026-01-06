# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains the React UI. `src/components` is for reusable UI pieces, `src/lib` for shared utilities, and `src/assets` for bundled static assets.
- `public/` holds static files served directly by Vite.
- `src-tauri/` contains the Rust backend and desktop packaging. `src-tauri/src` holds commands, `src-tauri/tauri.conf.json` is the main app config, and `src-tauri/capabilities` defines permissions.

## Build, Test, and Development Commands
- `pnpm install` installs dependencies for the workspace.
- `pnpm dev` runs the Vite dev server for the web UI.
- `pnpm tauri dev` runs the desktop app with the Rust backend and Vite.
- `pnpm build` runs TypeScript checks and produces the web build.
- `pnpm tauri build` packages the desktop app.
- `pnpm preview` serves the built web output locally.

## Coding Style & Naming Conventions
- TypeScript + React: use `.tsx` for components and `.ts` for utilities.
- Follow existing formatting: 2-space indent, double quotes, and semicolons.
- Component files use PascalCase (e.g., `TokenList.tsx`), hooks use `useX.ts`, and shared helpers live in `src/lib`.
- Avoid `any`; prefer precise types and keep functions focused.

## Testing Guidelines
- No test runner is configured yet. If you add tests, use `*.test.ts(x)` or `src/__tests__/` and add a `test` script in `package.json`.

## Commit & Pull Request Guidelines
- Git history is not present in this snapshot. Use `type(scope): summary` (e.g., `feat(ui): add token list`).
- PRs should include a clear description, test steps, and screenshots for UI changes. Link related issues when applicable.

## Security & Configuration Tips
- Keep Tauri permissions minimal; review changes in `src-tauri/capabilities`.
- Document any config changes in `src-tauri/tauri.conf.json` in the PR description.
