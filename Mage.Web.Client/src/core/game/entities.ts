/** The named entities the server's text uses, and what they stand for. */
const ENTITIES: Record<string, string> = {
  nbsp: ' ', amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", mdash: '—', ndash: '–', minus: '−', bull: '•',
  middot: '·', hellip: '…', lsquo: "'", rsquo: "'", ldquo: '"', rdquo: '"', times: '×', rarr: '→', larr: '←',
  laquo: '«', raquo: '»',
};

/**
 * Decodes HTML entities in one pass (so "&amp;lt;" stays "&lt;"). The server writes some without their semicolon
 * ("&bull Put a counter", from Modes and a few cards): a known name decodes either way (#204); anything else is left
 * as written.
 */
export function decodeEntities(text: string): string {
  return text.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);?/gi, (match, body: string) => {
    if (body[0] === '#') {
      const code = body[1] === 'x' || body[1] === 'X' ? parseInt(body.slice(2), 16) : Number(body.slice(1));
      return Number.isFinite(code) && code > 0 && code <= 0x10ffff ? String.fromCodePoint(code) : match;
    }
    return ENTITIES[body.toLowerCase()] ?? match;
  });
}
