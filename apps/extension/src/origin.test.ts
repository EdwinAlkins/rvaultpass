import assert from 'node:assert/strict';
import test from 'node:test';
import { safeHttpOrigin, sameDocument, originsMatch } from './origin';

test('une origine invalide ne vaut pas une chaîne vide', () => {
  assert.equal(safeHttpOrigin('javascript:alert(1)'), null);
  assert.equal(safeHttpOrigin('not a url'), null);
  assert.equal(safeHttpOrigin('https://example.com/login'), 'https://example.com');
  assert.notEqual(safeHttpOrigin('javascript:alert(1)'), '');
  assert.equal(originsMatch('javascript:alert(1)', 'data:text/html,hi'), false);
  assert.equal(originsMatch('', ''), false);
  assert.equal(originsMatch('https://example.com/a', 'https://example.com/b'), true);
});

test('un changement de document bloque le remplissage', () => {
  const before = { url: 'https://example.com/a', documentId: 'doc-1' };
  assert.equal(sameDocument(before, { url: 'https://example.com/b', documentId: 'doc-1' }), true);
  assert.equal(sameDocument(before, { url: 'https://other.example/a', documentId: 'doc-1' }), false);
  assert.equal(sameDocument(before, { url: 'https://example.com/a', documentId: 'doc-2' }), false);
});
