import { MessageType } from '../shared/protocol.js';

export class NetworkClient {
  constructor() {
    this.ws = null;
    this.connected = false;
    this.playerId = null;
    this.seed = null;
    this.remotePlayers = new Map();
    this.handlers = new Map();
    this.reconnectDelay = 2000;
  }

  on(type, handler) {
    this.handlers.set(type, handler);
  }

  connect(playerName = 'Player') {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const url = `${protocol}//${location.host}`;

    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      this.connected = true;
      this.send(MessageType.JOIN, { name: playerName });
    };

    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      const handler = this.handlers.get(msg.type);
      if (handler) handler(msg);
    };

    this.ws.onclose = () => {
      this.connected = false;
      setTimeout(() => this.connect(playerName), this.reconnectDelay);
    };

    this.ws.onerror = () => {
      this.ws?.close();
    };
  }

  send(type, payload = {}) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type, ...payload, t: Date.now() }));
    }
  }

  sendMove(x, y, z, yaw, pitch) {
    this.send(MessageType.MOVE, { x, y, z, yaw, pitch });
  }

  sendBreakBlock(x, y, z) {
    this.send(MessageType.BREAK_BLOCK, { x, y, z });
  }

  sendPlaceBlock(x, y, z, blockId) {
    this.send(MessageType.PLACE_BLOCK, { x, y, z, blockId });
  }

  sendAttackMob(mobId) {
    this.send(MessageType.ATTACK_MOB, { mobId });
  }

  disconnect() {
    this.ws?.close();
    this.ws = null;
    this.connected = false;
  }
}
