<script setup lang="ts">
import { computed } from 'vue';
import { isTauri, isMobile, platform } from '@/composables/useEnv';

/** Windows 桌面端即使切换手机布局，也保留顶部拖拽栏。 */
const forceDesktopBar = computed(() => isTauri && platform.value === 'Windows');

withDefaults(
  defineProps<{
    title?: string;
  }>(),
  {
    title: 'Legado',
  },
);
</script>

<template>
  <!-- 移动端：纯状态栏颜色遮罩，高度由 grid row（env safe-area-inset-top）决定，无文字 -->
  <header
    v-if="isMobile && !forceDesktopBar"
    class="title-bar title-bar--mobile"
    aria-hidden="true"
  />
  <!-- 桌面端：仅保留拖拽区域，不再渲染窗口控制按钮 -->
  <header v-else class="title-bar" data-tauri-drag-region>
    <span v-if="isMobile" class="title-bar__title">{{ title }}</span>
    <div class="title-bar__spacer" data-tauri-drag-region />
  </header>
</template>

<style scoped>
.title-bar {
  grid-area: title;
  display: flex;
  align-items: center;
  height: var(--topbar-height);
  padding-left: var(--space-4);
  background: transparent;
  user-select: none;
  -webkit-app-region: drag;
}

.title-bar__title {
  font-size: var(--fs-14);
  font-weight: var(--fw-semibold);
  color: var(--color-text);
  letter-spacing: 0.02em;
  flex-shrink: 0;
  -webkit-app-region: no-drag;
}

.title-bar__spacer {
  flex: 1;
  height: 100%;
  -webkit-app-region: drag;
}

/* ── 移动端顶栏：仅作状态栏背景遮盖，高度 = grid row (env safe-area-inset-top) ── */
.title-bar--mobile {
  background: transparent;
  border-bottom: none;
  -webkit-app-region: none;
}
</style>
