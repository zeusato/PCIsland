// Fails when vi.json and en.json do not have the same keys.
import { readFileSync } from 'node:fs';

const load = (locale) =>
  JSON.parse(readFileSync(new URL(`../src/lib/i18n/${locale}.json`, import.meta.url), 'utf8'));

const vi = Object.keys(load('vi'));
const en = Object.keys(load('en'));
const missingInEn = vi.filter((k) => !en.includes(k));
const missingInVi = en.filter((k) => !vi.includes(k));

if (missingInEn.length || missingInVi.length) {
  if (missingInEn.length) console.error('Missing in en.json:', missingInEn.join(', '));
  if (missingInVi.length) console.error('Missing in vi.json:', missingInVi.join(', '));
  process.exit(1);
}
console.log(`i18n ok (${vi.length} keys)`);
