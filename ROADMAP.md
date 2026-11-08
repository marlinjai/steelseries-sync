---
type: roadmap
---

# steelseries-sync Roadmap

## Leftovers

Dependency security exceptions and temporary overrides from the 2026-11-08 security update. Each line names when it can be closed.

- [ ] server: remove `"multer": "$multer"` from `server/package.json` overrides once `@nestjs/platform-express` 11.x depends on multer >=2.5.0 or the server moves to NestJS 12 (ES modules only, so a module-system migration). Until then the override forces multer 2.5.0 (security fix GHSA-q7qp-rjg7-q26m) under platform-express 11.2.8, which pins 2.4.0 exactly; `server/test/sync.e2e-spec.ts` proves multipart uploads still work (2026-11-08)
- [ ] server: remove the `minimatch@>=7.0.0 <11.0.1` override from `server/package.json` once glob 10/13, test-exclude 7 and @typescript-eslint/typescript-estree 8 accept minimatch >=11.0.1 (Dependabot alert #239, GHSA-g3hv-75wg-m249); it forces 11.0.5 outside their declared ^9/^10 ranges, verified by lint, build, unit, end-to-end and coverage runs (2026-11-08)
- [ ] server: deepmerge 4.3.1 (Dependabot alert #236, GHSA-gcqx-w48m-w2h4) has no patched release; it comes in only through jest-config and fork-ts-checker-webpack-plugin (@nestjs/cli), dev tooling that merges the repo's own config objects. Bump when deepmerge ships a fix or the parents drop it (2026-11-08)
- [ ] root: remove the `toml`, `katex` and `postcss-selector-parser` pnpm overrides from `package.json` once @marlinjai/clearify ships with remark-mdx-frontmatter 6 (smol-toml), mermaid 12 (katex >=0.18.2) and an @tailwindcss/typography release on postcss-selector-parser >=6.1.5 (Dependabot alerts #104, #126, #127, #111, #132) (2026-11-08)
- [ ] src-tauri: glib 0.18.5 (Dependabot alert #240, GHSA-wrw7-89jp-8q8g, unsound `VariantStrIter`) stays until Tauri 2 moves to glib >=0.20 or gtk4; the GTK stack is compiled only for Linux builds, the app ships as a macOS DMG, and the code never calls `VariantStrIter`. Run `cargo update` in `src-tauri` when tauri releases it (2026-11-08)
