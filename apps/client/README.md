# React + TypeScript + Vite

This template provides a minimal setup to get React working in Vite with HMR and some ESLint rules.

## Structure des dossiers (`src/`)

Convention décidée le 16/08/2026 (TPM-167) : le **type technique** est l'axe de rangement principal, le **domaine** (Parc vs interface générale) est secondaire, en sous-dossier à l'intérieur de chaque catégorie technique.

```
src/
  components/
    park/       Composants spécifiques à la feature Parc (Park.tsx, Toolbar.tsx, ...)
    shell/      Coquille d'interface générale (AppShell, TopBar, LeftNav, InspectorPanel, ...)
    common/     Composants génériques réutilisables, pas liés à une feature (ConfirmDialog, ...)
    ui/         Vendored shadcn/ui — ne pas modifier à la main, régénérer via `pnpm dlx shadcn`
  hooks/
    park/       Hooks React spécifiques au Parc (useParkSocket, useParkKeyboardShortcuts)
  lib/
    park/       Logique métier pure du Parc, sans React (commands, gridController, ...)
    utils.ts    Utilitaires génériques (ex. `cn`)
  types/
    park/       Types/interfaces du domaine Parc (tool.ts, selection.ts)
  rendering/    Utilitaires de rendu du canvas (grille, couleurs, conversions écran)
  assets/       Images statiques
```

Règle pour un nouveau fichier : d'abord choisir la catégorie technique (`components/`, `hooks/`, `lib/`, `types/`), puis le domaine (`park/` s'il est spécifique au Parc, sinon à la racine de la catégorie ou dans `common/`/`shell/` selon le cas). Les tests vivent dans un `__tests__/` à côté du module qu'ils couvrent, jamais centralisés ailleurs.

Currently, two official plugins are available:

- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react) uses [Oxc](https://oxc.rs)
- [@vitejs/plugin-react-swc](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react-swc) uses [SWC](https://swc.rs/)

## React Compiler

The React Compiler is not enabled on this template because of its impact on dev & build performances. To add it, see [this documentation](https://react.dev/learn/react-compiler/installation).

## Expanding the ESLint configuration

If you are developing a production application, we recommend updating the configuration to enable type-aware lint rules:

```js
export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...

      // Remove tseslint.configs.recommended and replace with this
      tseslint.configs.recommendedTypeChecked,
      // Alternatively, use this for stricter rules
      tseslint.configs.strictTypeChecked,
      // Optionally, add this for stylistic rules
      tseslint.configs.stylisticTypeChecked,

      // Other configs...
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])

```

You can also install [eslint-plugin-react-x](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-x) and [eslint-plugin-react-dom](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-dom) for React-specific lint rules:

```js
// eslint.config.js
import reactX from 'eslint-plugin-react-x'
import reactDom from 'eslint-plugin-react-dom'

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...
      // Enable lint rules for React
      reactX.configs['recommended-typescript'],
      // Enable lint rules for React DOM
      reactDom.configs.recommended,
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])

```
