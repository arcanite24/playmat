/// <reference types="vitest/config" />
import { readFileSync } from 'node:fs'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { pwa } from './scripts/pwa-plugin'
import { releaseNotes } from './scripts/release-notes-plugin'

/** Libraries that change rarely get their own chunks, so an app release doesn't re-download them. */
const VENDOR_CHUNKS: [chunk: string, packages: RegExp][] = [
  ['vendor-react', /\/node_modules\/(react|react-dom|scheduler)\//],
  ['vendor-router', /\/node_modules\/(react-router|react-router-dom)\//],
  ['vendor-radix', /\/node_modules\/@radix-ui\//],
  ['vendor-query', /\/node_modules\/@tanstack\//],
  ['vendor-zustand', /\/node_modules\/(zustand|use-sync-external-store)\//],
  // the icon runtime and the icons the entry uses: the lazy icons chunk below needs the runtime without taking it in
  ['vendor-lucide', /\/node_modules\/lucide-react\/dist\/esm\/(?!icons\/)/],
]

/**
 * Icons that only lazy screens use share one chunk: on their own they came out as two dozen chunks of a few hundred
 * bytes each, mostly chunk overhead. Icons the entry imports still load with it (in vendor-lucide), so the first load
 * doesn't grow. (Computed once per build: `vite build --watch` keeps the first build's list.)
 */
const ICON_FILE = /\/node_modules\/lucide-react\/dist\/esm\/icons\/([\w-]+)\.mjs$/
const LUCIDE_IMPORT = /import\s*\{([^}]*)\}\s*from\s*["']lucide-react["']/g

type ModuleGraph = { getModuleInfo(id: string): { code: string | null; importedIds: readonly string[] } | null; getModuleIds(): IterableIterator<string> }

let entryIcons: Set<string> | null = null
/** the icon files (as named in lucide-react/dist/esm/icons) that modules loaded with the entry import */
function iconsOfEntry(graph: ModuleGraph): Set<string> {
  if (entryIcons) return entryIcons
  // lucide-react's index: export { default as Star, default as StarIcon, ... } from './icons/star.mjs'
  const fileOf = new Map<string, string>()
  const index = readFileSync(new URL('./node_modules/lucide-react/dist/esm/lucide-react.mjs', import.meta.url), 'utf8')
  for (const [, names, file] of index.matchAll(/export \{([^}]*)\} from '\.\/icons\/([\w-]+)\.mjs'/g)) {
    for (const name of names.split(',')) fileOf.set(name.replace(/default as/, '').trim(), file)
  }
  const entry = [...graph.getModuleIds()].find((id) => id.replace(/\\/g, '/').endsWith('/src/app/main.tsx'))
  const icons = new Set<string>()
  const seen = new Set<string>()
  const stack = entry ? [entry] : []
  while (stack.length) {
    const id = stack.pop()!
    if (seen.has(id) || id.includes('/node_modules/')) continue
    seen.add(id)
    const info = graph.getModuleInfo(id)
    for (const [, names] of (info?.code ?? '').matchAll(LUCIDE_IMPORT)) {
      for (const name of names.split(',')) {
        const file = fileOf.get(name.split(' as ')[0].trim())
        if (file) icons.add(file)
      }
    }
    stack.push(...(info?.importedIds ?? []))
  }
  entryIcons = icons
  return icons
}

// the product name is a build setting (src/app/brand.ts); index.html's title reads it too
const appName = process.env.VITE_APP_NAME?.trim() || 'Playmat'
process.env.VITE_APP_NAME = appName
// so is the version (the image workflow passes the release tag or sha-<7>); index.html's app-version meta reads it,
// and the deploy's smoke test checks it. Empty: a development build, "dev" in the app.
const appVersion = process.env.VITE_APP_VERSION?.trim() || ''
process.env.VITE_APP_VERSION = appVersion

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), pwa(appName), releaseNotes(appVersion)],
  build: {
    // scripts/check-bundle-size.mjs reads the manifest to find each page's chunks
    manifest: true,
    rollupOptions: {
      input: {
        main: 'index.html',
      },
      output: {
        manualChunks(id, graph) {
          const path = id.replace(/\\/g, '/')
          const icon = ICON_FILE.exec(path)?.[1]
          // (not icons/index.mjs, which imports them all: a manual chunk takes in its unassigned imports)
          if (icon && icon !== 'index' && !iconsOfEntry(graph).has(icon)) return 'icons'
          return VENDOR_CHUNKS.find(([, packages]) => packages.test(path))?.[0]
        },
      },
    },
  },
  test: {
    projects: [
      {
        // pure logic: stores, protocol, services. A .ts test that needs a DOM opts in per file with
        // `// @vitest-environment happy-dom`.
        extends: true,
        test: {
          name: 'node',
          include: ['src/**/*.test.ts'],
          environment: 'node',
        },
      },
      {
        // React components and hooks, rendered with @testing-library/react in happy-dom
        extends: true,
        test: {
          name: 'dom',
          include: ['src/**/*.test.tsx'],
          environment: 'happy-dom',
          setupFiles: ['src/test/setupDom.ts'],
        },
      },
    ],
  },
})
