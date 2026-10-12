import type { PasswordGenerationParams } from '@rvaultpass/vault-client';

/** Demande envoyée par le popup ou la page d'options au service worker. */
export type ExtensionRequest =
  | { type: 'status' }
  | { type: 'save-server'; serverUrl: string }
  | { type: 'unlock'; password: string }
  | { type: 'lock' }
  | { type: 'search'; query: string }
  | { type: 'matches'; origin: string }
  | { type: 'fill'; entryId: string; tabId: number }
  | { type: 'credential'; entryId: string }
  | { type: 'generate'; params: Partial<PasswordGenerationParams> };

/** Identifiant exposé au popup, sans mot de passe. */
export interface PublicEntry {
  id: string;
  title: string;
  username: string;
  url: string;
  origin: string;
}

/** Réponse du service worker. `error` est présent seulement en cas d'échec. */
export interface ExtensionResponse {
  error?: string;
  unlocked?: boolean;
  serverUrl?: string;
  ok?: boolean;
  count?: number;
  entries?: PublicEntry[];
  password?: string;
}

/** Message reçu par le script de contenu pour remplir un formulaire. */
export interface FillRequest {
  type: 'fill';
  username: string;
  password: string;
  origin: string;
}

const REQUEST_TYPES = [
  'status',
  'save-server',
  'unlock',
  'lock',
  'search',
  'matches',
  'fill',
  'credential',
  'generate',
] as const;

/** Indique qu'un message du navigateur est une demande connue. */
export function isExtensionRequest(value: unknown): value is ExtensionRequest {
  if (!value || typeof value !== 'object' || !('type' in value)) return false;
  const type = value.type;
  return typeof type === 'string' && (REQUEST_TYPES as readonly string[]).includes(type);
}
