import esbuild from 'esbuild';
import { solidPlugin } from 'esbuild-plugin-solid';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';

const watch = process.argv.includes('--watch');
mkdirSync('dist', { recursive: true });
const ctx = await esbuild.context({
  entryPoints: ['src/main.tsx'],
  bundle: true,
  format: 'iife',
  target: 'es2020',
  plugins: [solidPlugin()],
  alias: {
    '@tauri-apps/api/core': './src/silver.ts',
    '@tauri-apps/api/event': './src/silver.ts',
  },
  conditions: ['browser'],
  define: { 'process.env.NODE_ENV': watch ? '"development"' : '"production"' },
  minify: !watch,
  outfile: 'dist/app.js',
  logLevel: 'info',
});
if (watch) { await ctx.watch(); console.log('obserwuję zmiany…'); }
else { await ctx.rebuild(); await ctx.dispose(); }
