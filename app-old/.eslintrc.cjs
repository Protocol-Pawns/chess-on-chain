/** @type { import("eslint").Linter.Config } */
module.exports = {
  root: false,
  extends: ["plugin:svelte/recommended"],
  parserOptions: {
    extraFileExtensions: [".svelte"],
  },
  overrides: [
    {
      files: ["*.svelte"],
      parser: "svelte-eslint-parser",
      parserOptions: {
        parser: "@typescript-eslint/parser",
      },
    },
  ],
  rules: {
    "import/no-unresolved": [
      "error",
      {
        // FIXME
        ignore: ["\\$app/.*", "\\$lib/.*", "svelte/.*"],
      },
    ],
  },
};
