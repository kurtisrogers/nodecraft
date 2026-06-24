import * as THREE from 'three';

export class RemotePlayerManager {
  constructor(scene) {
    this.scene = scene;
    this.players = new Map();
  }

  createMesh(name) {
    const group = new THREE.Group();

    const body = new THREE.Mesh(
      new THREE.BoxGeometry(0.6, 1.2, 0.3),
      new THREE.MeshLambertMaterial({ color: 0x3366cc })
    );
    body.position.y = 0.6;
    group.add(body);

    const head = new THREE.Mesh(
      new THREE.BoxGeometry(0.5, 0.5, 0.5),
      new THREE.MeshLambertMaterial({ color: 0xffcc99 })
    );
    head.position.y = 1.45;
    group.add(head);

    const label = this.createLabel(name);
    label.position.y = 2.1;
    group.add(label);

    return group;
  }

  createLabel(text) {
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 64;
    const ctx = canvas.getContext('2d');
    ctx.fillStyle = 'rgba(0,0,0,0.6)';
    ctx.fillRect(0, 0, 256, 64);
    ctx.fillStyle = '#fff';
    ctx.font = '28px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText(text, 128, 42);

    const texture = new THREE.CanvasTexture(canvas);
    const sprite = new THREE.Sprite(
      new THREE.SpriteMaterial({ map: texture, transparent: true })
    );
    sprite.scale.set(2, 0.5, 1);
    return sprite;
  }

  addOrUpdate(id, data) {
    let entry = this.players.get(id);
    if (!entry) {
      const mesh = this.createMesh(data.name || 'Player');
      this.scene.add(mesh);
      entry = { mesh, data };
      this.players.set(id, entry);
    }
    entry.data = { ...entry.data, ...data };
    if (!entry.data.name && data.name) entry.data.name = data.name;
    entry.mesh.position.set(data.x, data.y, data.z);
    entry.mesh.rotation.y = data.yaw ?? entry.data.yaw ?? 0;
  }

  remove(id) {
    const entry = this.players.get(id);
    if (entry) {
      this.scene.remove(entry.mesh);
      entry.mesh.traverse((child) => {
        if (child.geometry) child.geometry.dispose();
        if (child.material) {
          if (child.material.map) child.material.map.dispose();
          child.material.dispose();
        }
      });
      this.players.delete(id);
    }
  }

  sync(players, localId) {
    const seen = new Set();
    for (const p of players) {
      if (p.id === localId) continue;
      seen.add(p.id);
      this.addOrUpdate(p.id, p);
    }
    for (const id of this.players.keys()) {
      if (!seen.has(id)) this.remove(id);
    }
  }
}
