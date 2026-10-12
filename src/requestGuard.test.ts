import assert from 'node:assert/strict';
import test from 'node:test';
import { requestStillCurrent, revealStillCurrent, startRequest } from './requestGuard';

test('une réponse de recherche plus ancienne est ignorée', () => {
  const gate = { id: 0 };
  const first = startRequest(gate);
  const second = startRequest(gate);
  assert.equal(requestStillCurrent(first, gate), false);
  assert.equal(requestStillCurrent(second, gate), true);
});

test('un secret révélé ne s’applique plus si l’entrée a changé', () => {
  assert.equal(revealStillCurrent('entry-a', 'entry-a'), true);
  assert.equal(revealStillCurrent('entry-a', 'entry-b'), false);
  assert.equal(revealStillCurrent('entry-a', null), false);
});
