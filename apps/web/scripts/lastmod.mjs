// The content hash behind each sitemap `lastmod` (add-seo-and-discoverability).
// A page's date must move when its words change and not before, so a release
// that only rebuilds JavaScript does not tell crawlers the whole site changed.
import { createHash } from 'node:crypto';

/**
 * A page's words, with the build's own identity taken out: Astro's scoped
 * class names and island ids, the content hashes in island and stylesheet
 * URLs, and the Wasm build hash the tool island carries for problem reports.
 * These can move on a rebuild while the page still reads exactly the same.
 */
export const substantive = (html) =>
  (/<main[^>]*>([\s\S]*)<\/main>/.exec(html)?.[1] ?? html)
    .replace(/\s+/g, ' ')
    .replace(/(<astro-island\s+uid=")[^"]+(")/g, '$1$2')
    .replace(/astro-[a-z0-9]+/g, '')
    .replace(/(\/_astro\/[\w.-]*?)\.[A-Za-z0-9_-]{8}\.(js|css)\b/g, '$1.$2')
    .replace(/((?:&quot;|")buildHash(?:&quot;|")\s*:\s*(?:\[0,\s*)?(?:&quot;|"))[0-9a-f]{8,64}/g, '$1');

export const contentHash = (html) => createHash('sha256').update(substantive(html)).digest('hex').slice(0, 16);
