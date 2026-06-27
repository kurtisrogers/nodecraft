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
  if (new URLSearchParams(location.search).has('mobile')) return true;

  const coarsePointer = window.matchMedia('(pointer: coarse)').matches;
  const narrowScreen = window.matchMedia('(max-width: 900px)').matches;
  const touchCapable = 'ontouchstart' in window || navigator.maxTouchPoints > 0;
  const mobileUa = /Android|iPhone|iPad|iPod|Mobile|webOS|BlackBerry|IEMobile|Opera Mini/i.test(
    navigator.userAgent
  );

  return coarsePointer || (touchCapable && narrowScreen) || mobileUa;
}

export const BUILD_VERSION = '1.7.0';

const CHUNK_BLOCKS = 16;

/** View distance tuned per device — desktop sees ~144 blocks, mobile ~96. */
export function getRenderSettings() {
  const mobile = isMobileDevice();
  const renderDistance = mobile ? 6 : 9;
  const blockRadius = renderDistance * CHUNK_BLOCKS;

  return {
    mobile,
    renderDistance,
    cameraFar: mobile ? 320 : 420,
    fogNear: Math.round(blockRadius * 0.35),
    fogFar: Math.round(blockRadius * 1.65),
    pixelRatio: mobile ? 1.25 : 2,
    antialias: !mobile,
  };
}
