import { defineConfig } from 'vite';

// Deployed under a subpath (e.g. https://translunar.io/tools/frozen/) rather than
// domain root. ELFO_BASE lets the deploy pipeline set the real path; local dev and
// `vite preview` default to root.
export default defineConfig({
  base: process.env.ELFO_BASE ?? '/',
});
