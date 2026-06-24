import express from 'express';
import path from 'path';
import { fileURLToPath } from 'url';
import { createServer } from 'http';
import { WebSocketServer } from 'ws';
import { GameServer, handleMessage } from './gameServer.js';
import { MessageType } from './shared/protocol.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const app = express();
const server = createServer(app);
const wss = new WebSocketServer({ server });
const gameServer = new GameServer(Math.floor(Math.random() * 100000));
const PORT = process.env.PORT || 3000;

app.use(express.static(path.join(__dirname, '../public')));

app.get('/health', (_req, res) => {
  res.json({
    status: 'ok',
    game: 'nodecraft',
    players: gameServer.players.size,
    mobs: gameServer.mobs.size,
    seed: gameServer.seed,
  });
});

function broadcast(message, excludeId = null) {
  const data = JSON.stringify(message);
  for (const client of wss.clients) {
    if (client.readyState === 1 && client.clientId !== excludeId) {
      client.send(data);
    }
  }
}

wss.on('connection', (ws) => {
  ws.on('message', (raw) => {
    try {
      const msg = JSON.parse(raw.toString());
      handleMessage(gameServer, ws, ws.clientId, msg, broadcast);
    } catch {
      // ignore malformed messages
    }
  });

  ws.on('close', () => {
    if (ws.clientId) {
      gameServer.removePlayer(ws.clientId);
      broadcast({ type: MessageType.PLAYER_LEAVE, id: ws.clientId });
    }
  });
});

let lastTick = Date.now();
setInterval(() => {
  const now = Date.now();
  const dt = Math.min((now - lastTick) / 1000, 0.1);
  lastTick = now;
  gameServer.updateMobs(dt);
  if (gameServer.mobs.size > 0) {
    broadcast({ type: MessageType.MOBS_SYNC, mobs: [...gameServer.mobs.values()], dayTime: gameServer.dayTime });
  }
}, 100);

server.listen(PORT, () => {
  console.log(`Nodecraft running at http://localhost:${PORT}`);
  console.log(`World seed: ${gameServer.seed}`);
});
