import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import type { Plugin } from 'vite'

/** Files from public/ that belong to the app shell; precached by the service worker alongside the build output. */
const PUBLIC_SHELL = ['/favicon.svg', '/icon-192.png', '/icon-512.png', '/icon-maskable-512.png', '/apple-touch-icon.png']

// the mat's colours (src/app/styles/tokens.css): --mat-800 is the page background and the browser's theme colour
const THEME_COLOR = '#0f1714'
const BACKGROUND_COLOR = '#0f1714'

/** The web app manifest; the name is the build setting VITE_APP_NAME (src/app/brand.ts). */
export function webManifest(name: string) {
  return {
    name,
    short_name: name,
    description: 'Web client for XMage servers.',
    id: '/',
    start_url: '/',
    scope: '/',
    // a game wants the whole screen: no status bar over the table on Android; browsers without fullscreen fall back
    display: 'fullscreen',
    display_override: ['fullscreen', 'standalone'],
    theme_color: THEME_COLOR,
    background_color: BACKGROUND_COLOR,
    icons: [
      { src: '/icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'any' },
      { src: '/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any' },
      { src: '/icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
    ],
  }
}

/**
 * What the service worker precaches: the page, the build's JS and CSS (lazy screens included, so the shell works
 * offline wherever it is opened), the Barlow faces' latin woff2 files and the shell's public files. Other fonts and
 * pictures under /assets are cached on first use instead: fontsource ships many subsets and formats, and a page
 * downloads only the ones it needs. The pets' 3D scene (three.js, about 150 kB gzipped) is left out too: only players
 * at a table with a pet need it.
 */
export function precacheList(bundleFiles: Iterable<string>): string[] {
  const shell = /^assets\/(.+\.(js|css)|barlow(-condensed)?-latin-\d+-normal-.+\.woff2)$/
  const onDemand = /^assets\/PetScene-/
  const built = [...bundleFiles].filter((file) => shell.test(file) && !onDemand.test(file)).map((file) => `/${file}`)
  return ['/index.html', '/manifest.webmanifest', ...PUBLIC_SHELL, ...built.sort()]
}

/**
 * Makes the app installable: serves /manifest.webmanifest (dev and build) and, in builds, writes dist/sw.js from
 * scripts/sw.template.js with the version and precache list of that build. The list comes from the bundle itself
 * (the same files the build manifest lists, which Vite writes only after this plugin runs). The dev server has no
 * service worker; src/app/pwa.ts registers it in production builds only.
 */
export function pwa(appName: string): Plugin {
  const manifest = `${JSON.stringify(webManifest(appName), null, 2)}\n`
  return {
    name: 'playmat-pwa',
    enforce: 'post',
    configureServer(server) {
      server.middlewares.use('/manifest.webmanifest', (_request, response) => {
        response.setHeader('Content-Type', 'application/manifest+json')
        response.end(manifest)
      })
    },
    generateBundle(_options, bundle) {
      this.emitFile({ type: 'asset', fileName: 'manifest.webmanifest', source: manifest })
      const precache = precacheList(Object.keys(bundle))
      const template = readFileSync(new URL('./sw.template.js', import.meta.url), 'utf8')
      const html = bundle['index.html']
      const page = html?.type === 'asset' ? String(html.source) : ''
      const version = createHash('sha256').update(template).update(manifest).update(page).update(precache.join('\n')).digest('hex').slice(0, 12)
      const source = template.replace('__VERSION__', () => JSON.stringify(version)).replace('__PRECACHE__', () => JSON.stringify(precache, null, 2))
      this.emitFile({ type: 'asset', fileName: 'sw.js', source })
    },
  }
}
