/** Origine http(s) utilisable pour une comparaison de sécurité. */
export function safeHttpOrigin(url: string | null | undefined): string | null {
  if (!url) return null;
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return null;
    if (!parsed.hostname) return null;
    return parsed.origin;
  } catch {
    return null;
  }
}

/** Deux origines http(s) identiques. Une origine invalide ne correspond à rien, pas même à une autre origine invalide. */
export function originsMatch(page: string | null | undefined, entry: string | null | undefined): boolean {
  const left = safeHttpOrigin(page);
  return left !== null && left === safeHttpOrigin(entry);
}

/** Le document ciblé n'a pas changé entre la vérification et l'injection. */
export function sameDocument(
  before: { url?: string; documentId?: string },
  after: { url?: string; documentId?: string },
): boolean {
  const origin = safeHttpOrigin(before.url);
  if (!origin || origin !== safeHttpOrigin(after.url)) return false;
  if (before.documentId && after.documentId && before.documentId !== after.documentId) return false;
  return true;
}
