import * as THREE from 'three';
import { BlockId } from './blocks.js';
import { BLOCK_DROPS, isBlockItem } from './items.js';
import { World } from './world.js';
import { WorldRenderer, HighlightBox } from './renderer.js';
import { Player } from './player.js';
import { MobManager } from './mobs.js';
import { NetworkClient } from './network.js';
import { RemotePlayerManager } from './remotePlayers.js';
import { GameUI } from './ui.js';
import { MessageType } from './protocol-shim.js';
import { isStaticDeploy, isMobileDevice } from './config.js';
import { TouchControls } from './touchControls.js';
import { WeatherSystem } from './weather.js';

class Game {
  constructor() {
    this.canvas = document.getElementById('game-canvas');
    this.loadingEl = document.getElementById('loading');
    this.fpsEl = document.getElementById('fps');
    this.posEl = document.getElementById('position');
    this.clock = new THREE.Clock();
    this.frameCount = 0;
    this.fpsTimer = 0;
    this.networkMoveTimer = 0;
    this.init();
  }

  init() {
    this.renderer = new THREE.WebGLRenderer({ canvas: this.canvas, antialias: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.setSize(window.innerWidth, window.innerHeight);
    this.renderer.setClearColor(0x87ceeb);

    this.scene = new THREE.Scene();
    this.scene.fog = new THREE.Fog(0x87ceeb, 40, 120);

    this.camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 200);

    this.ambientLight = new THREE.AmbientLight(0xffffff, 0.6);
    this.scene.add(this.ambientLight);
    this.sunLight = new THREE.DirectionalLight(0xffffff, 0.8);
    this.sunLight.position.set(50, 100, 30);
    this.scene.add(this.sunLight);

    this.weather = new WeatherSystem(this.scene, this.camera);

    this.world = new World(42);
    this.worldRenderer = new WorldRenderer(this.scene, this.world);
    this.highlight = new HighlightBox(this.scene);
    this.mobManager = new MobManager(this.scene, this.world);
    this.remotePlayers = new RemotePlayerManager(this.scene);

    this.player = new Player(this.camera, this.world);
    this.player.setupControls(this.canvas);
    this.player.onPrimaryAction = () => this.primaryAction();
    this.player.onPlaceBlock = () => this.placeBlock();
    this.player.onHotbarChange = () => this.ui.refreshHotbar();
    this.player.onToggleInventory = () => this.toggleInventory();
    this.player.onInventoryOpen = () => this.ui.isOpen();

    this.ui = new GameUI(this);

    this.giveStarterItems();

    this.world.loadChunksAround(0, 0);
    this.worldRenderer.update(0, 0);
    this.player.spawn();
    this.player.updateSelectedBlock();
    this.ui.refreshHotbar();
    this.worldRenderer.update(this.player.position.x, this.player.position.z);

    this.setupNetwork();

    if (document.getElementById('mobile-controls') && isMobileDevice()) {
      this.touchControls = new TouchControls(this.player, this);
      this.touchControls.init();
    }

    window.addEventListener('resize', () => this.onResize());
    this.loadingEl.classList.add('hidden');
    this.animate();
  }

  giveStarterItems() {
    const inv = this.player.inventory;
    inv.addItem(BlockId.DIRT, 16);
    inv.addItem(BlockId.COBBLESTONE, 16);
    inv.addItem(BlockId.WOOD, 8);
  }

  resetWorld(seed, blockChanges, mobs, dayTime, players, playerId) {
    this.worldRenderer.dispose();
    this.world = new World(seed);
    this.world.applyModifications(blockChanges);
    this.worldRenderer = new WorldRenderer(this.scene, this.world);
    this.player.world = this.world;
    this.mobManager.world = this.world;
    this.mobManager.syncFromServer(mobs, this.scene);
    this.weather.syncDayTime(dayTime ?? 0);
    this.mobManager.dayTime = this.weather.dayTime;
    this.remotePlayers.sync(players, playerId);
    this.ui.setPlayerCount(players.length);
    this.world.loadChunksAround(0, 0);
    this.worldRenderer.update(0, 0);
    this.player.spawn();
    this.worldRenderer.update(this.player.position.x, this.player.position.z);
  }

  setupNetwork() {
    this.network = new NetworkClient();
    const name = `Player${Math.floor(Math.random() * 1000)}`;
    this.player.name = name;

    this.network.onOffline = () => {
      this.ui.setPlayerCount(1, isStaticDeploy() ? 'Single-player (GitHub Pages)' : 'Single-player');
      this.mobManager.authoritative = true;
    };

    this.network.on(MessageType.WELCOME, (msg) => {
      this.player.id = msg.playerId;
      this.resetWorld(msg.seed, msg.blockChanges, msg.mobs, msg.dayTime, msg.players, msg.playerId);
    });

    this.network.on(MessageType.PLAYER_JOIN, (msg) => {
      this.remotePlayers.addOrUpdate(msg.player.id, msg.player);
      this.ui.setPlayerCount(this.remotePlayers.players.size + 1);
    });

    this.network.on(MessageType.PLAYER_LEAVE, (msg) => {
      this.remotePlayers.remove(msg.id);
      this.ui.setPlayerCount(this.remotePlayers.players.size + 1);
    });

    this.network.on(MessageType.PLAYER_MOVE, (msg) => {
      this.remotePlayers.addOrUpdate(msg.id, msg);
    });

    this.network.on(MessageType.BLOCK_CHANGE, (msg) => {
      this.world.setBlock(msg.x, msg.y, msg.z, msg.blockId);
      this.worldRenderer.rebuildChunkAt(msg.x, msg.z);
      this.worldRenderer.update(this.player.position.x, this.player.position.z);
    });

    this.network.on(MessageType.MOBS_SYNC, (msg) => {
      this.mobManager.syncFromServer(msg.mobs, this.scene);
      if (msg.dayTime !== undefined) this.weather.syncDayTime(msg.dayTime);
    });

    this.network.on(MessageType.MOB_UPDATE, (msg) => {
      if (msg.drops) {
        for (const drop of msg.drops) {
          this.player.inventory.addItem(drop.itemId, drop.count);
        }
        this.ui.refreshHotbar();
      }
      const mob = this.mobManager.mobs.get(msg.mob?.id);
      if (mob && msg.mob && !msg.mob.alive) {
        mob.alive = false;
      }
    });

    this.network.connect(name);
  }

  toggleInventory() {
    this.ui.toggleInventory();
  }

  primaryAction() {
    if (this.player.attackCooldown > 0) return;

    const origin = this.camera.position.clone();
    const direction = this.player.getLookDirection();
    const mob = this.mobManager.raycast(origin, direction);
    if (mob) {
      this.player.attackCooldown = 0.4;
      if (this.network.connected) {
        this.network.sendAttackMob(mob.id);
      } else {
        const result = this.mobManager.attack(mob.id);
        if (result?.killed) {
          for (const drop of result.drops) {
            this.player.inventory.addItem(drop.itemId, drop.count);
          }
          this.ui.refreshHotbar();
        }
      }
      return;
    }

    this.breakBlock();
  }

  breakBlock() {
    const hit = this.player.raycast();
    if (!hit) return;
    const { x, y, z } = hit.block;
    const blockId = this.world.getBlock(x, y, z);
    if (blockId === BlockId.BEDROCK || blockId === BlockId.LAVA) return;

    const drop = BLOCK_DROPS[blockId];
    if (drop) this.player.inventory.addItem(drop, 1);

    if (this.network.connected) {
      this.network.sendBreakBlock(x, y, z);
    }
    this.world.setBlock(x, y, z, BlockId.AIR);
    this.worldRenderer.rebuildChunkAt(x, z);
    this.ui.refreshHotbar();
  }

  placeBlock() {
    const itemId = this.player.inventory.getHotbarItem(this.player.hotbarIndex);
    if (!itemId || !isBlockItem(itemId)) return;
    if (!this.player.inventory.hasItem(itemId, 1)) return;

    const hit = this.player.raycast();
    if (!hit) return;
    const { x, y, z } = hit.face;
    const px = Math.floor(this.player.position.x);
    const py = Math.floor(this.player.position.y);
    const pz = Math.floor(this.player.position.z);

    if (x === px && y >= py && y <= py + 1 && z === pz) return;

    this.player.inventory.removeItem(itemId, 1);
    if (this.network.connected) {
      this.network.sendPlaceBlock(x, y, z, itemId);
    }
    this.world.setBlock(x, y, z, itemId);
    this.worldRenderer.rebuildChunkAt(x, z);
    this.player.updateSelectedBlock();
    this.ui.refreshHotbar();
  }

  onResize() {
    this.camera.aspect = window.innerWidth / window.innerHeight;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(window.innerWidth, window.innerHeight);
  }

  animate() {
    requestAnimationFrame(() => this.animate());

    const dt = Math.min(this.clock.getDelta(), 0.05);
    this.player.update(dt);

    if (!this.network.connected) {
      this.mobManager.update(dt, this.player.position);
    } else {
      this.networkMoveTimer += dt;
      if (this.networkMoveTimer > 0.05) {
        this.networkMoveTimer = 0;
        const p = this.player.position;
        this.network.sendMove(p.x, p.y, p.z, this.player.yaw, this.player.pitch);
      }
      this.mobManager.dayTime = this.weather.dayTime;
    }

    const env = this.weather.update(dt, this.player.position);
    this.mobManager.dayTime = this.weather.dayTime;
    this.ui.setEnvironment(env);
    this.ambientLight.intensity = env.ambientIntensity;
    this.sunLight.intensity = env.sunIntensity;
    this.renderer.setClearColor(env.skyColor);
    this.scene.fog.color.copy(env.fogColor);

    const hit = this.player.raycast();
    if (hit && this.player.isControlling() && !this.ui.open) {
      this.highlight.show(hit.block.x, hit.block.y, hit.block.z);
    } else {
      this.highlight.hide();
    }

    if (Math.random() < 0.15) {
      this.worldRenderer.update(this.player.position.x, this.player.position.z);
    }

    this.renderer.render(this.scene, this.camera);

    this.frameCount++;
    this.fpsTimer += dt;
    if (this.fpsTimer >= 1) {
      this.fpsEl.textContent = `${this.frameCount} FPS`;
      this.frameCount = 0;
      this.fpsTimer = 0;
    }

    const p = this.player.position;
    this.posEl.textContent = `${Math.floor(p.x)}, ${Math.floor(p.y)}, ${Math.floor(p.z)}`;
  }
}

new Game();
