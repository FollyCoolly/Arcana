<script lang="ts">
  import * as THREE from 'three';
  import { CSS3DRenderer, CSS3DObject } from 'three/examples/jsm/renderers/CSS3DRenderer.js';
  import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
  import type { SkillWithLevel } from '$lib/types/skill';

  let {
    skills,
    onCardClick,
  }: {
    skills: SkillWithLevel[];
    onCardClick: (skill: SkillWithLevel) => void;
  } = $props();

  let containerEl = $state<HTMLDivElement | undefined>(undefined);

  const ROMAN = ['0', 'I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X'];

  function seededRandom(index: number, salt: number): number {
    const x = Math.sin(index * 127.1 + salt * 311.7) * 43758.5453;
    return x - Math.floor(x);
  }

  function computePositions(count: number): { x: number; y: number; z: number }[] {
    const BASE_RADIUS = 550;
    const result: { x: number; y: number; z: number }[] = [];

    for (let i = 0; i < count; i++) {
      const skill = skills[i];
      const levelFrac = skill.current_level / Math.max(1, skill.skill.max_level);
      const radialBias = 1.0 - levelFrac * 0.55;

      const theta = Math.acos(1 - 2 * seededRandom(i, 0));
      const phi = 2 * Math.PI * seededRandom(i, 1);
      const flattenY = 0.55;
      const r = BASE_RADIUS * radialBias * (0.7 + 0.3 * seededRandom(i, 2));

      result.push({
        x: r * Math.sin(theta) * Math.cos(phi),
        y: r * Math.sin(theta) * Math.sin(phi) * flattenY,
        z: r * Math.cos(theta),
      });
    }
    return result;
  }

  function buildCardElement(skill: SkillWithLevel): HTMLDivElement {
    const leveled = skill.current_level > 0;
    const progressPct = skill.max_points > 0
      ? (skill.current_points / skill.max_points) * 100
      : 0;

    const el = document.createElement('div');
    el.className = `rm-tarot-card rm-nebula-card${leveled ? ' rm-tarot-card--leveled' : ''}`;
    el.style.width = '160px';

    el.innerHTML = `
      <div class="rm-tarot-card-inner">
        <div class="rm-tarot-top">
          <span class="rm-tarot-level">${ROMAN[skill.current_level] ?? skill.current_level}</span>
          <span class="rm-tarot-pack">${escapeHtml(skill.pack_name)}</span>
        </div>
        <div class="rm-tarot-art">
          <div class="rm-tarot-star-stack">
            <div class="rm-tarot-star rm-ts-1"></div>
            <div class="rm-tarot-star rm-ts-2"></div>
            <div class="rm-tarot-star rm-ts-3"></div>
            <div class="rm-tarot-star rm-ts-4"></div>
            <div class="rm-tarot-star rm-ts-5"></div>
          </div>
          <div class="rm-tarot-stripe"></div>
        </div>
        <div class="rm-tarot-name-strip">
          <span class="rm-tarot-name">${escapeHtml(skill.skill.name)}</span>
        </div>
        <div class="rm-tarot-bottom">
          <div class="rm-tarot-progress">
            <div class="rm-tarot-progress-fill" style="width:${progressPct}%"></div>
          </div>
          <span class="rm-tarot-lv">LV ${skill.current_level}</span>
        </div>
      </div>
    `;

    return el;
  }

  function escapeHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }

  $effect(() => {
    const el = containerEl;
    if (!el || skills.length === 0) return;

    // Scene
    const scene = new THREE.Scene();
    const W = el.clientWidth;
    const H = el.clientHeight;

    const camera = new THREE.PerspectiveCamera(50, W / H, 1, 5000);
    camera.position.set(0, 0, 1400);

    // CSS3DRenderer
    const renderer = new CSS3DRenderer();
    renderer.setSize(W, H);
    renderer.domElement.style.position = 'absolute';
    renderer.domElement.style.inset = '0';
    el.appendChild(renderer.domElement);

    // OrbitControls
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.08;
    controls.rotateSpeed = 0.55;
    controls.zoomSpeed = 0.9;
    controls.minDistance = 200;
    controls.maxDistance = 2800;
    controls.enablePan = false;

    // Card element → skill map for event delegation
    const cardSkillMap = new Map<HTMLElement, SkillWithLevel>();

    // Drag vs click: track on the container (where OrbitControls captures pointer)
    let pointerDownPos = { x: 0, y: 0 };
    let isDragging = false;
    let pointerDownTarget: EventTarget | null = null;

    el.addEventListener('pointerdown', (e: PointerEvent) => {
      pointerDownPos = { x: e.clientX, y: e.clientY };
      pointerDownTarget = e.target;
      isDragging = false;
    });
    el.addEventListener('pointermove', (e: PointerEvent) => {
      if (!isDragging) {
        const dx = e.clientX - pointerDownPos.x;
        const dy = e.clientY - pointerDownPos.y;
        if (dx * dx + dy * dy > 25) isDragging = true;
      }
    });
    el.addEventListener('pointerup', (e: PointerEvent) => {
      if (isDragging) return;

      // Find which card was clicked — check both pointerdown target and pointerup target
      const target = (pointerDownTarget as HTMLElement) ?? (e.target as HTMLElement);
      const cardEl = target.closest?.('.rm-nebula-card') as HTMLElement | null;
      if (!cardEl) return;

      const skill = cardSkillMap.get(cardEl);
      if (!skill) return;

      onCardClick(skill);
    });

    // Create cards with per-card random wobble offsets
    const positions = computePositions(skills.length);
    const objects: CSS3DObject[] = [];
    const wobbles: { x: number; y: number; z: number }[] = [];

    for (let i = 0; i < skills.length; i++) {
      const skill = skills[i];
      const cardEl = buildCardElement(skill);

      cardSkillMap.set(cardEl, skill);

      const obj = new CSS3DObject(cardEl);
      const pos = positions[i];
      obj.position.set(pos.x, pos.y, pos.z);

      wobbles.push({
        x: (seededRandom(i, 3) - 0.5) * 0.25,
        y: (seededRandom(i, 4) - 0.5) * 0.3,
        z: (seededRandom(i, 5) - 0.5) * 0.12,
      });

      scene.add(obj);
      objects.push(obj);
    }

    // Animation loop — billboard cards toward camera + wobble
    let animFrameId: number;
    function animate() {
      animFrameId = requestAnimationFrame(animate);
      controls.update();

      for (let i = 0; i < objects.length; i++) {
        const obj = objects[i];
        obj.lookAt(camera.position);
        obj.rotation.x += wobbles[i].x;
        obj.rotation.y += wobbles[i].y;
        obj.rotation.z += wobbles[i].z;
      }

      renderer.render(scene, camera);
    }
    animate();

    // Resize
    const resizeObserver = new ResizeObserver(() => {
      const nW = el.clientWidth;
      const nH = el.clientHeight;
      camera.aspect = nW / nH;
      camera.updateProjectionMatrix();
      renderer.setSize(nW, nH);
    });
    resizeObserver.observe(el);

    return () => {
      cancelAnimationFrame(animFrameId);
      controls.dispose();
      resizeObserver.disconnect();
      for (const obj of objects) {
        obj.element.remove();
      }
      renderer.domElement.remove();
      cardSkillMap.clear();
    };
  });
</script>

<div class="rm-nebula-viewport" bind:this={containerEl}></div>

<style>
  .rm-nebula-viewport {
    position: absolute;
    inset: 0;
    overflow: hidden;
    cursor: grab;
  }
  .rm-nebula-viewport:active {
    cursor: grabbing;
  }
</style>
