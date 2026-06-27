import * as THREE from 'three';
import { isMobileDevice } from './config.js';

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
const WEATHER_BLEND_SEC = 4;

export class WeatherSystem {
  constructor(scene, camera) {
    this.scene = scene;
    this.camera = camera;
    this.type = WeatherType.CLEAR;
    this.targetType = WeatherType.CLEAR;
    this.dayTime = 0;
    this.weatherTimer = 30;
    this.weatherBlend = 1;
    this.flashTimer = 0;
    this.thunderTimer = 0;
    this.particleCount = isMobileDevice() ? 280 : 600;
    this.velocities = new Float32Array(this.particleCount);
    this.skyColor = new THREE.Color();
    this.fogColor = new THREE.Color();
    this.clearSky = new THREE.Color();
    this.overcastSky = new THREE.Color();
    this.rainSky = new THREE.Color(0x556677);
    this.snowSky = new THREE.Color(0x99aabb);
    this.flashSky = new THREE.Color(0xccccff);
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

    if (roll < 0.35) this.targetType = WeatherType.CLEAR;
    else if (roll < 0.6) this.targetType = WeatherType.RAIN;
    else if (roll < 0.8 && cold) this.targetType = WeatherType.SNOW;
    else if (roll < 0.9) this.targetType = WeatherType.THUNDER;
    else this.targetType = cold ? WeatherType.SNOW : WeatherType.RAIN;

    this.weatherTimer = 45 + Math.random() * 60;
    this.weatherBlend = 0;
  }

  getPrecipIntensity(type) {
    switch (type) {
      case WeatherType.RAIN:
      case WeatherType.THUNDER:
        return 1;
      case WeatherType.SNOW:
        return 0.85;
      default:
        return 0;
    }
  }

  update(dt, playerPos) {
    this.dayTime += dt;
    this.weatherTimer -= dt;
    if (this.weatherTimer <= 0) this.pickNextWeather();

    if (this.weatherBlend < 1) {
      this.weatherBlend = Math.min(1, this.weatherBlend + dt / WEATHER_BLEND_SEC);
      if (this.weatherBlend >= 1) this.type = this.targetType;
    }

    const currentPrecip = this.getPrecipIntensity(this.type) * (1 - this.weatherBlend)
      + this.getPrecipIntensity(this.targetType) * this.weatherBlend;
    const isPrecip = currentPrecip > 0.05;
    this.particles.visible = isPrecip;

    if (isPrecip) {
      const isSnow = this.targetType === WeatherType.SNOW
        || (this.type === WeatherType.SNOW && this.weatherBlend < 1);
      this.particles.material = isSnow ? this.snowMaterial : this.rainMaterial;

      const positions = this.particles.geometry.attributes.position;
      const px = playerPos.x;
      const py = playerPos.y + 10;
      const pz = playerPos.z;

      for (let i = 0; i < this.particleCount; i++) {
        let x = positions.getX(i);
        let y = positions.getY(i);
        let z = positions.getZ(i);

        y -= this.velocities[i] * dt * (isSnow ? 0.35 : 1);
        x += isSnow ? Math.sin(this.dayTime + i) * 0.02 : -0.05;
        z += isSnow ? Math.cos(this.dayTime + i) * 0.02 : 0;

        if (y < playerPos.y - 2) {
          y = py + Math.random() * 15;
          x = px + (Math.random() - 0.5) * 50;
          z = pz + (Math.random() - 0.5) * 50;
        }

        positions.setXYZ(i, x, y, z);
      }
      positions.needsUpdate = true;
    }

    if (this.type === WeatherType.THUNDER || this.targetType === WeatherType.THUNDER) {
      this.thunderTimer -= dt;
      if (this.thunderTimer <= 0) {
        this.thunderTimer = 4 + Math.random() * 12;
        this.flashTimer = 0.15 + Math.random() * 0.1;
      }
    }

    if (this.flashTimer > 0) this.flashTimer -= dt;

    return this.getEnvironment(currentPrecip);
  }

  getEnvironment(precipIntensity = 0) {
    const sun = this.sunHeight;
    const night = this.isNight;

    if (night) {
      this.clearSky.setHex(0x0a0a20);
    } else if (sun > 0) {
      this.clearSky.setHSL(0.58, 0.6, 0.45 + sun * 0.25);
    } else {
      this.clearSky.setHex(0x1a1040);
    }

    this.overcastSky.copy(this.clearSky);
    if (precipIntensity > 0) {
      const overcast = this.targetType === WeatherType.SNOW ? this.snowSky : this.rainSky;
      this.overcastSky.lerp(overcast, 0.55 * precipIntensity);
    }

    this.skyColor.copy(this.overcastSky);
    if (this.flashTimer > 0) {
      this.skyColor.lerp(this.flashSky, 0.7);
    }

    this.fogColor.copy(this.skyColor);
    const ambient = night ? 0.25 : 0.55;
    const sunIntensity = Math.max(0.1, sun) * (precipIntensity < 0.2 ? 0.85 : 0.45);

    let timeLabel = night ? 'Night' : 'Day';
    if (sun > 0 && sun < 0.25 && !night) timeLabel = 'Sunset';
    if (sun > 0 && sun < 0.25 && night) timeLabel = 'Sunrise';

    const activeType = this.weatherBlend >= 1 ? this.type : this.targetType;

    return {
      skyColor: this.skyColor,
      fogColor: this.fogColor,
      ambientIntensity: ambient + (this.flashTimer > 0 ? 0.5 : 0),
      sunIntensity,
      isNight: night,
      timeLabel,
      weatherLabel: WEATHER_LABELS[activeType],
      dayTime: this.dayTime,
    };
  }

  syncDayTime(dayTime) {
    if (dayTime !== undefined) this.dayTime = dayTime;
  }
}
