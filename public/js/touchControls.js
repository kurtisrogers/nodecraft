export class TouchControls {
  constructor(player, game) {
    this.player = player;
    this.game = game;
    this.container = document.getElementById('mobile-controls');
    this.joystick = document.getElementById('joystick');
    this.knob = document.getElementById('joystick-knob');
    this.startBtn = document.getElementById('mobile-start');
    this.joystickTouchId = null;
    this.lookTouchId = null;
    this.joystickCenter = { x: 0, y: 0 };
    this.joystickRadius = 50;
    this.lookTouch = { x: 0, y: 0 };
    this.enabled = false;
    this.started = false;
  }

  init() {
    if (!this.container || !this.startBtn) {
      console.warn('Nodecraft: mobile controls markup not found');
      return;
    }

    document.body.classList.add('mobile');
    this.container.classList.remove('hidden');
    document.getElementById('desktop-instructions')?.classList.add('hidden');
    document.getElementById('mobile-instructions')?.classList.remove('hidden');

    const start = (e) => {
      e?.preventDefault();
      e?.stopPropagation();
      if (this.started) return;
      this.startGame();
    };
    this.startBtn.addEventListener('touchend', start, { passive: false });
    this.startBtn.addEventListener('click', start);

    this.setupJoystick();
    this.setupLookZone();
    this.setupButtons();
    this.setupHotbarTouch();
  }

  canUseGameplayControls() {
    return this.enabled && !this.game.ui.isOpen();
  }

  startGame() {
    this.started = true;
    this.enabled = true;
    this.player.mobileActive = true;
    document.body.classList.add('playing');
    this.startBtn.classList.add('hidden');
    document.getElementById('instructions')?.classList.add('hidden');

    this.player.spawn();
    this.game.world.loadChunksAround(this.player.position.x, this.player.position.z);
    this.game.worldRenderer.update(this.player.position.x, this.player.position.z);
  }

  resetJoystick() {
    this.joystickTouchId = null;
    this.player.touchMove = { x: 0, z: 0 };
    if (this.knob) this.knob.style.transform = 'translate(-50%, -50%)';
  }

  setupJoystick() {
    const onStart = (e) => {
      if (!this.canUseGameplayControls()) return;
      e.preventDefault();
      const touch = [...e.changedTouches].find(
        (t) => this.joystick.contains(t.target)
      );
      if (!touch || this.joystickTouchId !== null) return;

      this.joystickTouchId = touch.identifier;
      const rect = this.joystick.getBoundingClientRect();
      this.joystickCenter = {
        x: rect.left + rect.width / 2,
        y: rect.top + rect.height / 2,
      };
      this.joystickRadius = rect.width / 2 - 20;
      this.updateJoystick(touch.clientX, touch.clientY);
    };

    const onMove = (e) => {
      if (this.joystickTouchId === null) return;
      if (!this.canUseGameplayControls()) {
        this.resetJoystick();
        return;
      }
      const touch = [...e.changedTouches].find((t) => t.identifier === this.joystickTouchId);
      if (!touch) return;
      e.preventDefault();
      this.updateJoystick(touch.clientX, touch.clientY);
    };

    const onEnd = (e) => {
      const touch = [...e.changedTouches].find((t) => t.identifier === this.joystickTouchId);
      if (!touch) return;
      this.resetJoystick();
    };

    this.joystick.addEventListener('touchstart', onStart, { passive: false });
    document.addEventListener('touchmove', onMove, { passive: false });
    document.addEventListener('touchend', onEnd);
    document.addEventListener('touchcancel', onEnd);
  }

  updateJoystick(clientX, clientY) {
    let dx = clientX - this.joystickCenter.x;
    let dy = clientY - this.joystickCenter.y;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (dist > this.joystickRadius) {
      dx = (dx / dist) * this.joystickRadius;
      dy = (dy / dist) * this.joystickRadius;
    }

    this.knob.style.transform = `translate(calc(-50% + ${dx}px), calc(-50% + ${dy}px))`;

    const nx = dx / this.joystickRadius;
    const ny = dy / this.joystickRadius;
    this.player.touchMove = { x: nx, z: ny };
  }

  setupLookZone() {
    const sensitivity = 0.004;

    const onStart = (e) => {
      if (!this.canUseGameplayControls() || this.lookTouchId !== null) return;
      const touch = e.changedTouches[0];
      if (!touch) return;
      if (touch.target.closest('.mobile-btn, #joystick, #hotbar, .inv-slot, #menu-overlay, .menu-panel, #btn-close-menu')) return;

      const inLookZone = touch.clientX > window.innerWidth * 0.35;
      if (!inLookZone) return;

      e.preventDefault();
      this.lookTouchId = touch.identifier;
      this.lookTouch = { x: touch.clientX, y: touch.clientY };
    };

    const onMove = (e) => {
      if (this.lookTouchId === null) return;
      if (!this.canUseGameplayControls()) {
        this.lookTouchId = null;
        return;
      }
      const touch = [...e.changedTouches].find((t) => t.identifier === this.lookTouchId);
      if (!touch) return;
      e.preventDefault();

      const dx = touch.clientX - this.lookTouch.x;
      const dy = touch.clientY - this.lookTouch.y;
      this.lookTouch = { x: touch.clientX, y: touch.clientY };
      this.player.addLookDelta(-dx * sensitivity, -dy * sensitivity);
    };

    const onEnd = (e) => {
      const touch = [...e.changedTouches].find((t) => t.identifier === this.lookTouchId);
      if (touch) this.lookTouchId = null;
    };

    document.addEventListener('touchstart', onStart, { passive: false });
    document.addEventListener('touchmove', onMove, { passive: false });
    document.addEventListener('touchend', onEnd);
    document.addEventListener('touchcancel', onEnd);
  }

  setupButtons() {
    const bind = (id, onDown, onUp) => {
      const el = document.getElementById(id);
      if (!el) return;

      el.addEventListener('touchstart', (e) => {
        if (!this.enabled) return;
        e.preventDefault();
        e.stopPropagation();
        onDown();
        el.classList.add('pressed');
      }, { passive: false });

      const release = (e) => {
        e.preventDefault();
        onUp?.();
        el.classList.remove('pressed');
      };

      el.addEventListener('touchend', release, { passive: false });
      el.addEventListener('touchcancel', release, { passive: false });
    };

    bind('btn-jump', () => {
      if (!this.canUseGameplayControls()) return;
      this.player.touchJump = true;
    }, () => { this.player.touchJump = false; });

    bind('btn-sprint', () => {
      if (!this.canUseGameplayControls()) return;
      this.player.touchSprint = true;
    }, () => { this.player.touchSprint = false; });

    bind('btn-break', () => {
      if (!this.canUseGameplayControls()) return;
      this.game.primaryAction();
    });

    bind('btn-place', () => {
      if (!this.canUseGameplayControls()) return;
      this.game.placeBlock();
    });

    bind('btn-inventory', () => {
      if (!this.enabled) return;
      this.resetJoystick();
      this.game.toggleInventory();
    });
  }

  setupHotbarTouch() {
    const hotbar = document.getElementById('hotbar');
    hotbar.addEventListener('touchstart', (e) => {
      if (!this.canUseGameplayControls()) return;
      const slot = e.target.closest('.hotbar-slot');
      if (!slot) return;
      e.preventDefault();
      e.stopPropagation();
      const index = parseInt(slot.dataset.index, 10);
      if (!Number.isNaN(index)) {
        this.player.hotbarIndex = index;
        this.player.updateSelectedBlock();
        this.game.ui.refreshHotbar();
      }
    }, { passive: false });
  }
}
