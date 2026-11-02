// Builds the focus sounds release (docs/guides/focus-sounds-suno.md §3).
// Usage: node scripts/focus-sounds-manifest.mjs <source-dir> --release focus-sounds-v1 [--license "..."]
// <source-dir>/<pack>/pack.json + *.mp3  →  <source-dir>/release/<pack>-<file>.mp3 + manifest.json
import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const args = process.argv.slice(2);
const option = (name) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : undefined; };
const source = args[0] && !args[0].startsWith('--') ? resolve(args[0]) : null;
const release = option('--release');
const license = option('--license') ?? '';
const safe = (value) => /^[a-z0-9_-][a-z0-9._-]{0,95}$/.test(value);
const fail = (message) => { console.error(`✗ ${message}`); process.exit(1); };

if (!source || !release) fail('usage: node scripts/focus-sounds-manifest.mjs <source-dir> --release focus-sounds-v1 [--license "..."]');
if (!safe(release)) fail(`release tag "${release}" may only use a-z 0-9 . _ -`);

const out = join(source, 'release');
rmSync(out, { recursive: true, force: true });
mkdirSync(out);

const packs = [];
let total = 0;
for (const id of readdirSync(source).sort()) {
  const dir = join(source, id);
  if (id === 'release' || !statSync(dir).isDirectory()) continue;
  if (!safe(id)) fail(`pack folder "${id}" may only use a-z 0-9 . _ -`);
  let meta;
  try { meta = JSON.parse(readFileSync(join(dir, 'pack.json'), 'utf8')); } catch { fail(`${id}/pack.json is missing or invalid`); }
  if (!['focus', 'break'].includes(meta.role)) fail(`${id}/pack.json: role must be "focus" or "break"`);
  if (!meta.name?.vi || !meta.name?.en) fail(`${id}/pack.json: name needs "vi" and "en"`);
  const files = readdirSync(dir).filter((f) => f.toLowerCase().endsWith('.mp3')).sort();
  if (!files.length) fail(`${id}: no .mp3 files (the app cannot play Opus)`);
  const tracks = files.map((file) => {
    const lower = file.toLowerCase();
    if (!safe(lower)) fail(`${id}/${file}: rename to a-z 0-9 . _ - (no spaces or accents)`);
    const published = `${id}-${lower}`;
    const bytes = readFileSync(join(dir, file));
    if (bytes.length > 50 * 1024 * 1024) fail(`${id}/${file} is larger than 50 MiB`);
    copyFileSync(join(dir, file), join(out, published));
    total += bytes.length;
    const fallback = lower.replace(/\.mp3$/, '').replace(/^\d+[-_ ]*/, '').replace(/[-_]+/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
    return { file: published, title: meta.titles?.[file] ?? fallback, size: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') };
  });
  packs.push({ id, role: meta.role, name: { vi: meta.name.vi, en: meta.name.en }, tracks });
  console.log(`✓ ${id} (${meta.role}): ${tracks.length} tracks`);
}
if (!packs.length) fail('no pack folders found');
writeFileSync(join(out, 'manifest.json'), JSON.stringify({ schema: 1, release, license, packs }, null, 2) + '\n');
console.log(`\n${packs.reduce((n, p) => n + p.tracks.length, 0)} tracks, ${(total / 1024 / 1024).toFixed(1)} MiB → ${out}`);
console.log(`\nPublish:\n  gh release create ${release} "${out.replace(/\\/g, '/')}"/* --repo zeusato/pcisland-sounds --title "Focus sounds ${release}" --notes "Focus music for PCisland"`);
