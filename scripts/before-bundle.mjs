#!/usr/bin/env node
// Tauri's macOS bundler auto-includes every [[bin]] from Cargo.toml but only
// lipo-merges the main binary for universal-apple-darwin. This hook lipo-merges
// the secondary `claude-usage` CLI so bundling can find it. No-ops on Linux,
// Windows, and single-arch macOS builds.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const targetDir = resolve(repoRoot, 'src-tauri', 'target');

const arm = resolve(targetDir, 'aarch64-apple-darwin', 'release', 'claude-usage');
const x64 = resolve(targetDir, 'x86_64-apple-darwin', 'release', 'claude-usage');

if (!existsSync(arm) || !existsSync(x64)) {
  process.exit(0);
}

const outDir = resolve(targetDir, 'universal-apple-darwin', 'release');
const out = resolve(outDir, 'claude-usage');
mkdirSync(outDir, { recursive: true });
execFileSync('lipo', ['-create', arm, x64, '-output', out], { stdio: 'inherit' });
console.log(`lipo-merged claude-usage -> ${out}`);
