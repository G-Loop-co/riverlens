import { test } from 'node:test';
import assert from 'node:assert/strict';
import { signingEnv } from './signing-env.mjs';
test('absent CI secrets do not request certificate import or notarization', () => {
  const source = {APPLE_CERTIFICATE:'', APPLE_CERTIFICATE_PASSWORD:'', APPLE_ID:'', APPLE_PASSWORD:'', APPLE_TEAM_ID:'', APPLE_SIGNING_IDENTITY:'-', OTHER:''};
  assert.deepEqual(signingEnv(source), {APPLE_SIGNING_IDENTITY:'-', OTHER:''});
  assert.equal(source.APPLE_CERTIFICATE, '');
});
test('configured signing credentials are preserved verbatim', () => {
  const source = {APPLE_CERTIFICATE:'fixture-base64', APPLE_CERTIFICATE_PASSWORD:'fixture', APPLE_SIGNING_IDENTITY:'Developer ID Application: Fixture', APPLE_ID:'fixture@example.test', APPLE_PASSWORD:'fixture', APPLE_TEAM_ID:'fixture'};
  assert.deepEqual(signingEnv(source), source);
});
