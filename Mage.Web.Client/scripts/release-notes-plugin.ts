import { existsSync, readFileSync } from 'node:fs'
import type { Plugin } from 'vite'
import { parseReleaseNotes } from '../src/core/util/releaseNotes.ts'

/**
 * Writes dist/release-notes.json, the "What's new" dialog's notes (src/app/whatsNew.ts), from release-notes.md beside
 * package.json: the GitHub release's body, which the image workflow (.github/workflows/images.yml) puts there before a
 * release build. No file, or a build without a version (VITE_APP_VERSION), writes nothing, and the app shows no dialog.
 * RELEASE_NOTES_URL, when set, is the release page the dialog links to.
 */
export function releaseNotes(version: string): Plugin {
  const file = new URL('../release-notes.md', import.meta.url)
  return {
    name: 'playmat-release-notes',
    apply: 'build',
    generateBundle() {
      if (!version || !existsSync(file)) return
      const notes = parseReleaseNotes(readFileSync(file, 'utf8'), version, process.env.RELEASE_NOTES_URL?.trim() || undefined)
      if (notes.sections.length === 0) return
      this.emitFile({ type: 'asset', fileName: 'release-notes.json', source: `${JSON.stringify(notes, null, 2)}\n` })
    },
  }
}
