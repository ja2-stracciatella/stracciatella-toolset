import { defineConfig, globalIgnores } from 'eslint/config';
import { fixupPluginRules } from '@eslint/compat';
import typescriptEslint from '@typescript-eslint/eslint-plugin';
import typescriptParser from '@typescript-eslint/parser';
import importPlugin from 'eslint-plugin-import';
import promisePlugin from 'eslint-plugin-promise';
import reactPlugin from 'eslint-plugin-react';
import reactHooksPlugin from 'eslint-plugin-react-hooks';
import jsxA11yPlugin from 'eslint-plugin-jsx-a11y';
import prettierPlugin from 'eslint-plugin-prettier';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import webpackEslintConfig from './.erb/configs/webpack.config.eslint.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

export default defineConfig([
  // Base JavaScript configuration
  {
    files: ['**/*.{js,jsx}'],
    plugins: {
      import: fixupPluginRules(importPlugin),
      promise: fixupPluginRules(promisePlugin),
      prettier: prettierPlugin,
    },
    languageOptions: {
      ecmaVersion: 2020,
      sourceType: 'module',
      globals: {
        process: 'readonly',
        console: 'readonly',
        Buffer: 'readonly',
        __dirname: 'readonly',
        __filename: 'readonly',
        module: 'readonly',
        require: 'readonly',
        exports: 'readonly',
        global: 'readonly',
      },
    },
    rules: {
      // Import rules
      'import/no-unresolved': 'error',

      'import/extensions': [
        'error',
        'ignorePackages',
        {
          js: 'never',
          jsx: 'never',
          ts: 'never',
          tsx: 'never',
        },
      ],

      // General rules
      'no-param-reassign': ['error', { props: false }],
      'no-unused-vars': 'warn',

      // Prettier
      'prettier/prettier': 'error',
    },
    settings: {
      'import/resolver': {
        node: {},
        webpack: {
          config: webpackEslintConfig,
        },
      },
    },
  },

  // React JSX configuration
  {
    files: ['**/*.{jsx,tsx}'],
    plugins: {
      react: fixupPluginRules(reactPlugin),
      'react-hooks': fixupPluginRules(reactHooksPlugin),
      'jsx-a11y': fixupPluginRules(jsxA11yPlugin),
    },
    languageOptions: {
      parserOptions: {
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
    settings: {
      react: {
        version: 'detect',
      },
    },
    rules: {
      // React rules
      'react/jsx-filename-extension': [1, { extensions: ['.jsx', '.tsx'] }],

      // React hooks
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',
    },
  },

  // TypeScript configuration for source files
  {
    files: ['src/**/*.{ts,tsx}', 'assets/**/*.{ts,tsx}'],
    plugins: {
      '@typescript-eslint': fixupPluginRules(typescriptEslint),
      import: fixupPluginRules(importPlugin),
      promise: fixupPluginRules(promisePlugin),
      prettier: prettierPlugin,
    },
    languageOptions: {
      parser: typescriptParser,
      parserOptions: {
        ecmaVersion: 2020,
        sourceType: 'module',
        project: './tsconfig.json',
        tsconfigRootDir: __dirname,
        createDefaultProgram: true,
      },
    },
    rules: {
      // Import rules
      'import/no-unresolved': 'error',
      'import/extensions': [
        'error',
        'ignorePackages',
        {
          js: 'never',
          jsx: 'never',
          ts: 'never',
          tsx: 'never',
        },
      ],

      // General rules
      'no-param-reassign': ['error', { props: false }],

      // Prettier
      'prettier/prettier': 'error',
    },
    settings: {
      'import/resolver': {
        node: {},
        webpack: {
          config: webpackEslintConfig,
        },
        typescript: {},
      },
      'import/parsers': {
        '@typescript-eslint/parser': ['.ts', '.tsx'],
      },
    },
  },

  // Configuration for build/config TypeScript files
  {
    files: ['.erb/**/*.{ts,tsx}', 'vitest.config.ts', 'eslint.config.js'],
    plugins: {
      '@typescript-eslint': fixupPluginRules(typescriptEslint),
      import: fixupPluginRules(importPlugin),
      promise: fixupPluginRules(promisePlugin),
      prettier: prettierPlugin,
    },
    languageOptions: {
      parser: typescriptParser,
      parserOptions: {
        ecmaVersion: 2020,
        sourceType: 'module',
      },
      globals: {
        process: 'readonly',
        console: 'readonly',
        Buffer: 'readonly',
        __dirname: 'readonly',
        __filename: 'readonly',
        module: 'readonly',
        require: 'readonly',
        exports: 'readonly',
        global: 'readonly',
      },
    },
    rules: {
      // Import rules (relaxed)
      'import/no-unresolved': 'off',
      'import/extensions': 'off',

      // General rules (relaxed for config files)
      'global-require': 'off',
      'no-console': 'off',
      'no-unused-vars': 'off',

      // Prettier
      'prettier/prettier': 'error',
    },
    settings: {
      'import/resolver': {
        node: {},
        typescript: {},
      },
    },
  },

  // Global ignores
  globalIgnores([
    '**/logs',
    '**/*.log',
    '**/pids',
    '**/*.pid',
    '**/*.seed',
    '**/coverage',
    '**/.eslintcache',
    '**/node_modules',
    '**/.DS_Store',
    'release/app/dist',
    'release/build',
    '.erb/dll',
    '**/.idea',
    '**/npm-debug.log.*',
    '**/*.css.d.ts',
    '**/*.sass.d.ts',
    '**/*.scss.d.ts',
  ]),
]);
