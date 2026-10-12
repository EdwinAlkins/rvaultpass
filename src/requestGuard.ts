/** Identifie une recherche lancée, pour ignorer une réponse devenue obsolète. */
export interface RequestGate {
  id: number;
}

export function startRequest(gate: RequestGate): number {
  gate.id += 1;
  return gate.id;
}

export function requestStillCurrent(started: number, gate: RequestGate): boolean {
  return started === gate.id;
}

/** Un secret révélé ne s'applique que si l'entrée demandée est encore sélectionnée. */
export function revealStillCurrent(requestedId: string, selectedId: string | null): boolean {
  return selectedId === requestedId;
}
