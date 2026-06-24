import * as THREE from 'three';
import { BlockId, HOTBAR_BLOCKS, BLOCKS } from './blocks.js';
import { World } from './world.js';
import { WorldRenderer, HighlightBox } from './renderer.js';
import { Player } from './player.js';

class Game {
  constructor() {
    this.canvas = document.getElementById('game-canvas');
    this.loadingEl = document.getElementById('loading');
    this.overlayEl = document.getElementById('overlay');
    this.hotbarEl = document.getElementById('hotbar');
    this.fpsEl = document.getElementById('fps');
    this.posEl = document.getElementById('position');
    this.clock = new THREE.Clock();
    this.frameCount = 0;
    this.fpsTimer = 0;
    this.init();
  }

  init() {
    this.renderer = new THREE.WebGLRenderer({
      canvas: this.canvas,
      antialias: true,
    });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.setSize(window.innerWidth, window.innerHeight);
    this.renderer.setClearColor(0x87ceeb);

    this.scene = new THREE.Scene();
    this.scene.fog = new THREE.Fog(0x87ceeb, 40, 120);

    this.camera = new THREE.PerspectiveCamera(
      75,
      window.innerWidth / window.innerHeight,
      0.1,
      200
    );

    const ambient = new THREE.AmbientLight(0xffffff, 0.6);
    this.scene.add(ambient);

    const sun = new THREE.DirectionalLight(0xffffff, 0.8);
    sun.position.set(50, 100, 30);
    this.scene.add(sun);

    this.world = new World(Math.floor(Math.random() * 100000));
    this.worldRenderer = new WorldRenderer(this.scene, this.world);
    this.highlight = new HighlightBox(this.scene);

    this.player = new Player(this.camera, this.world);
    this.player.setupControls(this.canvas);

    this.player.onBreakBlock = () => this.breakBlock();
    this.player.onPlaceBlock = () => this.placeBlock();
    this.player.onHotbarChange = (index, blockId) => this.updateHotbar(index, blockId);

    this.buildHotbar();
    this.player.spawn();
    this.worldRenderer.update(this.player.position.x, this.player.position.z);

    window.addEventListener('resize', () => this.onResize());

    this.loadingEl.classList.add('hidden');
    this.animate();
  }

  buildHotbar() {
    this.hotbarEl.innerHTML = '';
    HOTBAR_BLOCKS.forEach((blockId, i) => {
      const slot = document.createElement('div');
      slot.className = 'hotbar-slot' + (i === 0 ? ' selected' : '');
      slot.dataset.index = i;

      const block = BLOCKS[blockId];
      const color = typeof block.color === 'number'
        ? `#${block.color.toString(16).padStart(6, '0')}`
        : `#${block.color.side.toString(16).padStart(6, '0')}`;

      slot.innerHTML = `
        <div class="block-preview" style="background:${color}"></div>
        <span class="slot-number">${i + 1}</span>
      `;
      this.hotbarEl.appendChild(slot);
    });
  }

  updateHotbar(index) {
    const slots = this.hotbarEl.querySelectorAll('.hotbar-slot');
    slots.forEach((slot, i) => {
      slot.classList.toggle('selected', i === index);
    });
  }

  breakBlock() {
    const hit = this.player.raycast();
    if (!hit) return;
    const { x, y, z } = hit.block;
    if (this.world.getBlock(x, y, z) === BlockId.BEDROCK) return;
    this.world.setBlock(x, y, z, BlockId.AIR);
    this.worldRenderer.rebuildChunkAt(x, z);
    this.worldRenderer.update(this.player.position.x, this.player.position.z);
  }

  placeBlock() {
    const hit = this.player.raycast();
    if (!hit) return;
    const { x, y, z } = hit.face;
    const px = Math.floor(this.player.position.x);
    const py = Math.floor(this.player.position.y);
    const pz = Math.floor(this.player.position.z);

    if (x === px && y >= py && y <= py + 1 && z === pz) return;
    if (x === px && y === py + 1 && z === pz) return;

    this.world.setBlock(x, y, z, this.player.selectedBlock);
    this.worldRenderer.rebuildChunkAt(x, z);
    this.worldRenderer.update(this.player.position.x, this.player.position.z);
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

    const hit = this.player.raycast();
    if (hit && this.player.pointerLocked) {
      this.highlight.show(hit.block.x, hit.block.y, hit.block.z);
    } else {
      this.highlight.hide();
    }

    if (Math.random() < 0.05) {
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
