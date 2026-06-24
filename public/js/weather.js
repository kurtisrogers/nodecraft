import * as THREE from 'three';

export const WeatherType = {
  CLEAR: 'clear',
  RAIN: 'rain',
  SNOW: 'snow',
  THUNDER: 'thunder',
};

const WEATHER_LABELS = {
  [WeatherType.CLEAR]: 'Clear',
  [WeatherType.RAIN]: 'Rain',
  [WeatherType.SNOW]: 'Snow',
  [WeatherType.THUNDER]: 'Thunderstorm',
};

const DAY_LENGTH = 240;

export class WeatherSystem {
  constructor(scene, camera) {
    this.scene = scene;
    this.camera = camera;
    this.type = WeatherType.CLEAR;
    this.dayTime = 0;
    this.weatherTimer = 30;
    this.flashTimer = 0;
    this.thunderTimer = 0;
    this.particleCount = 1200;
    this.velocities = new Float32Array(this.particleCount);
    this.initParticles();
  }

  initParticles() {
    const positions = new Float32Array(this.particleCount * 3);
    for (let i = 0; i < this.particleCount; i++) {
      positions[i * 3] = (Math.random() - 0.5) * 40;
      positions[i * 3 + 1] = Math.random() * 30 + 5;
      positions[i * 3 + 2] = (Math.random() - 0.5) * 40;
      this.velocities[i] = 8 + Math.random() * 6;
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));

    this.rainMaterial = new THREE.PointsMaterial({
      color: 0xaaccff,
      size: 0.12,
      transparent: true,
      opacity: 0.6,
      depthWrite: false,
    });

    this.snowMaterial = new THREE.PointsMaterial({
      color: 0xffffff,
      size: 0.2,
      transparent: true,
      opacity: 0.85,
      depthWrite: false,
    });

    this.particles = new THREE.Points(geometry, this.rainMaterial);
    this.particles.visible = false;
    this.particles.frustumCulled = false;
    this.scene.add(this.particles);
  }

  get isNight() {
    const cycle = (this.dayTime % DAY_LENGTH) / DAY_LENGTH;
    return cycle > 0.5;
  }

  get sunHeight() {
    const cycle = (this.dayTime % DAY_LENGTH) / DAY_LENGTH;
    return Math.sin(cycle * Math.PI * 2);
  }

  pickNextWeather() {
    const roll = Math.random();
    const cold = this.dayTime % DAY_LENGTH > DAY_LENGTH * 0.7;

    if (roll < 0.35) this.type = WeatherType.CLEAR;
    else if (roll < 0.6) this.type = WeatherType.RAIN;
    else if (roll < 0.8 && cold) this.type = WeatherType.SNOW;
    else if (roll < 0.9) this.type = WeatherType.THUNDER;
    else this.type = cold ? WeatherType.SNOW : WeatherType.RAIN;

    this.weatherTimer = 45 + Math.random() * 60;
  }

  update(dt, playerPos) {
    this.dayTime += dt;
    this.weatherTimer -= dt;
    if (this.weatherTimer <= 0) this.pickNextWeather();

    const isPrecip = this.type !== WeatherType.CLEAR;
    this.particles.visible = isPrecip;
    this.particles.material = this.type === WeatherType.SNOW ? this.snowMaterial : this.rainMaterial;

    if (isPrecip) {
      const positions = this.particles.geometry.attributes.position;
      const isSnow = this.type === WeatherType.SNOW;
      const px = playerPos.x;
      const py = playerPos.y + 10;
      const pz = playerPos.z;

      for (let i = 0; i < this.particleCount; i++) {
        let x = positions.getX(i);
        let y = positions.getY(i);
        let z = positions.getZ(i);

        y -= this.velocities[i] * dt * (isSnow ? 0.35 : 1);
        x += (isSnow ? Math.sin(this.dayTime + i) * 0.02 : -0.05);
        z += (isSnow ? Math.cos(this.dayTime + i) * 0.02 : 0);

        if (y < playerPos.y - 2) {
          y = py + Math.random() * 15;
          x = px + (Math.random() - 0.5) * 50;
          z = pz + (Math.random() - 0.5) * 50;
        }

        positions.setXYZ(i, x, y, z);
      }
      positions.needsUpdate = true;
      this.particles.position.set(0, 0, 0);
    }

    if (this.type === WeatherType.THUNDER) {
      this.thunderTimer -= dt;
      if (this.thunderTimer <= 0) {
        this.thunderTimer = 4 + Math.random() * 12;
        this.flashTimer = 0.15 + Math.random() * 0.1;
      }
    }

    if (this.flashTimer > 0) this.flashTimer -= dt;

    return this.getEnvironment();
  }

  getEnvironment() {
    const sun = this.sunHeight;
    const night = this.isNight;

    let sky = new THREE.Color();
    if (night) {
      sky.setHex(0x0a0a20);
    } else if (sun > 0) {
      sky.setHSL(0.58, 0.6, 0.45 + sun * 0.25);
    } else {
      sky.setHex(0x1a1040);
    }

    if (this.type === WeatherType.RAIN || this.type === WeatherType.THUNDER) {
      sky.lerp(new THREE.Color(0x556677), 0.55);
    } else if (this.type === WeatherType.SNOW) {
      sky.lerp(new THREE.Color(0x99aabb), 0.4);
    }

    if (this.flashTimer > 0) {
      sky.lerp(new THREE.Color(0xccccff), 0.7);
    }

    const fog = sky.clone();
    const ambient = night ? 0.25 : 0.55;
    const sunIntensity = Math.max(0.1, sun) * (this.type === WeatherType.CLEAR ? 0.85 : 0.45);

    let timeLabel = night ? 'Night' : 'Day';
    if (sun > 0 && sun < 0.25 && !night) timeLabel = 'Sunset';
    if (sun > 0 && sun < 0.25 && night) timeLabel = 'Sunrise';

    return {
      skyColor: sky,
      fogColor: fog,
      ambientIntensity: ambient + (this.flashTimer > 0 ? 0.5 : 0),
      sunIntensity,
      isNight: night,
      timeLabel,
      weatherLabel: WEATHER_LABELS[this.type],
      dayTime: this.dayTime,
    };
  }

  syncDayTime(dayTime) {
    if (dayTime !== undefined) this.dayTime = dayTime;
  }
}
