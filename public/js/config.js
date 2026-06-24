export function isGitHubPages() {
  return location.hostname.endsWith('.github.io');
}

export function isStaticDeploy() {
  if (isGitHubPages()) return true;
  return new URLSearchParams(location.search).has('static');
}

export function getWebSocketUrl() {
  if (isStaticDeploy()) return null;

  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${location.host}`;
}

export const DEPLOY_MODE = isStaticDeploy() ? 'static' : 'server';

export function isMobileDevice() {
  return (
    'ontouchstart' in window ||
    navigator.maxTouchPoints > 0 ||
    window.matchMedia('(pointer: coarse)').matches
  );
}
