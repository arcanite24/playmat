import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { CareerCatalogItem } from '../protocol/generated/views';
import { setCatalog } from '../app/ui/cosmeticsCatalog';

/** The server's cosmetics catalog (Mage.Server resources/career/cosmetics.json), as a signed-in client has it; tests run in the client's folder. */
export function loadTestCatalog(): CareerCatalogItem[] {
  const items: CareerCatalogItem[] = JSON.parse(readFileSync(resolve(process.cwd(), '../Mage.Server/src/main/resources/career/cosmetics.json'), 'utf8'));
  setCatalog(items);
  return items;
}
