export const MessageType = {
  JOIN: 'join',
  WELCOME: 'welcome',
  MOVE: 'move',
  PLAYER_MOVE: 'playerMove',
  PLAYER_JOIN: 'playerJoin',
  PLAYER_LEAVE: 'playerLeave',
  BREAK_BLOCK: 'breakBlock',
  PLACE_BLOCK: 'placeBlock',
  BLOCK_CHANGE: 'blockChange',
  ATTACK_MOB: 'attackMob',
  MOB_UPDATE: 'mobUpdate',
  MOBS_SYNC: 'mobsSync',
  CHAT: 'chat',
  PING: 'ping',
};

export function createMessage(type, payload = {}) {
  return JSON.stringify({ type, ...payload, t: Date.now() });
}

export function parseMessage(data) {
  return JSON.parse(data);
}
