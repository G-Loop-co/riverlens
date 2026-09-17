// GitHub expands absent secrets to empty strings. Tauri treats presence as configuration.
export function signingEnv(source) {
  const env = { ...source };
  for (const key of ['APPLE_CERTIFICATE', 'APPLE_CERTIFICATE_PASSWORD', 'APPLE_SIGNING_IDENTITY', 'APPLE_ID', 'APPLE_PASSWORD', 'APPLE_TEAM_ID', 'APPLE_API_KEY', 'APPLE_API_ISSUER', 'APPLE_API_KEY_PATH']) {
    if (env[key] === '') delete env[key];
  }
  return env;
}
