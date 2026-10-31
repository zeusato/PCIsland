import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';

export default ts.config(
  { ignores: ['dist/', 'src-tauri/', 'node_modules/', 'src/lib/ipc/bindings/'] },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: { parserOptions: { parser: ts.parser } },
  },
  {
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
      // Components must go through src/lib/ipc (docs/03-coding-principles.md §4).
      'no-restricted-imports': [
        'error',
        {
          paths: [
            { name: '@tauri-apps/api/core', message: 'Use $lib/ipc/commands instead.' },
            { name: '@tauri-apps/api/event', message: 'Use $lib/ipc/events instead.' },
          ],
        },
      ],
    },
  },
  {
    files: ['src/lib/ipc/**'],
    rules: { 'no-restricted-imports': 'off' },
  },
);
