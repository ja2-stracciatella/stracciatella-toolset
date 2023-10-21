module.exports = {
  extends: ['erb', 'plugin:@typescript-eslint/recommended'],
  rules: {
    // A temporary hack related to IDE not resolving correct package.json
    'import/no-extraneous-dependencies': 'off',
    'import/no-unresolved': 'error',
    // Since React 17 and typescript 4.1 you can safely disable the rule
    'react/react-in-jsx-scope': 'off',
    // Not autofixable, thus annoying
    'import/order': 'off',
    // imho named exports are better
    'import/prefer-default-export': 'off',
    // imho shadowing is fine with strict types
    '@typescript-eslint/no-shadow': 'off',
    // useful for forwarding props
    'react/jsx-props-no-spreading': 'off',
    // Below should be considered after migration
    'react/require-default-props': 'off',
    'react/no-array-index-key': 'off',
    'react/no-children-prop': 'off',
    '@typescript-eslint/ban-types': 'off',
    'no-plusplus': 'off',
    'no-shadow': 'off',
    'consistent-return': 'off',
    'jsx-a11y/click-events-have-key-events': 'off',
    'jsx-a11y/no-static-element-interactions': 'off',
    'promise/catch-or-return': 'off',
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
    'react/jsx-filename-extension': [1, { extensions: ['.jsx', '.tsx'] }],
    '@typescript-eslint/no-explicit-any': 'off',
    '@typescript-eslint/no-unused-vars': 'off',
  },
  parserOptions: {
    ecmaVersion: 2020,
    sourceType: 'module',
    project: './tsconfig.json',
    tsconfigRootDir: __dirname,
    createDefaultProgram: true,
  },
  settings: {
    'import/resolver': {
      // See https://github.com/benmosher/eslint-plugin-import/issues/1396#issuecomment-575727774 for line below
      node: {},
      webpack: {
        config: require.resolve('./.erb/configs/webpack.config.eslint.ts'),
      },
      typescript: {},
    },
    'import/parsers': {
      '@typescript-eslint/parser': ['.ts', '.tsx'],
    },
  },
};
