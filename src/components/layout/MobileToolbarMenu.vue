<script setup lang="ts">
/**
 * MobileToolbarMenu — 移动端工具栏三点菜单
 *
 * 桌面端：渲染默认 slot（完整工具栏按钮）
 * 移动端：收起为竖三点下拉菜单（AppSheet + AppListItem）
 *
 * 用法：
 *   <MobileToolbarMenu :options="menuOptions" @select="handleSelect">
 *     <!-- 桌面端工具栏按钮 -->
 *     <n-button .../>
 *   </MobileToolbarMenu>
 */
import { ChevronLeft, ChevronRight, MoreVertical } from 'lucide-vue-next';
import { computed, ref, type VNodeChild } from 'vue';
import { isMobile } from '../../composables/useEnv';
import AppListItem from '../base/AppListItem.vue';
import AppSheet from '../base/AppSheet.vue';

export interface MenuOption {
  label?: string | (() => VNodeChild);
  key?: string | number;
  disabled?: boolean;
  children?: readonly MenuOption[];
}

const props = defineProps<{
  options: MenuOption[];
}>();

const emit = defineEmits<{
  (e: 'select', key: string): void;
}>();

const sheetOpen = ref(false);
const menuStack = ref<
  Array<{
    title: string;
    options: MenuOption[];
  }>
>([]);

const currentMenu = computed(() => {
  return (
    menuStack.value.at(-1) ?? {
      title: '更多操作',
      options: props.options,
    }
  );
});

function isLeafOption(option: MenuOption): boolean {
  return !Array.isArray(option.children) || option.children.length === 0;
}

function getOptionChildren(option: MenuOption): MenuOption[] {
  return Array.isArray(option.children) ? [...option.children] : [];
}

function optionTitle(option: MenuOption): string {
  if (typeof option.label === 'string') {
    return option.label;
  }
  return String(option.key ?? '未命名操作');
}

function openSheet() {
  menuStack.value = [
    {
      title: '更多操作',
      options: props.options,
    },
  ];
  sheetOpen.value = true;
}

function closeSheet() {
  sheetOpen.value = false;
  menuStack.value = [];
}

function openChildMenu(option: MenuOption) {
  const children = getOptionChildren(option);
  if (!children.length) {
    return;
  }
  menuStack.value.push({
    title: optionTitle(option),
    options: children,
  });
}

function goBack() {
  if (menuStack.value.length <= 1) {
    closeSheet();
    return;
  }
  menuStack.value.pop();
}

function onSelect(key: string) {
  closeSheet();
  emit('select', key);
}
</script>

<template>
  <!-- 桌面端：直接展示 slot 内容 -->
  <template v-if="!isMobile">
    <slot />
  </template>
  <!-- 移动端：三点菜单触发 AppSheet 底部抽屉 -->
  <template v-else>
    <n-button size="small" quaternary aria-label="更多操作" @click="openSheet">
      <template #icon>
        <MoreVertical :size="16" />
      </template>
    </n-button>
    <AppSheet v-model="sheetOpen" @close="closeSheet">
      <div class="mtm-sheet" role="menu">
        <div class="mtm-sheet__header">
          <button
            v-if="menuStack.length > 1"
            class="mtm-sheet__back"
            type="button"
            @click="goBack"
          >
            <ChevronLeft :size="16" />
            <span>返回</span>
          </button>
          <div class="mtm-sheet__title">{{ currentMenu.title }}</div>
          <div class="mtm-sheet__header-spacer" aria-hidden="true" />
        </div>
        <AppListItem
          v-for="opt in currentMenu.options"
          :key="String(opt.key)"
          :title="optionTitle(opt)"
          :disabled="!!opt.disabled"
          role="menuitem"
          @click="
            isLeafOption(opt) ? onSelect(String(opt.key)) : openChildMenu(opt)
          "
        >
          <template v-if="!isLeafOption(opt)" #trailing>
            <ChevronRight :size="16" />
          </template>
        </AppListItem>
      </div>
    </AppSheet>
  </template>
</template>

<style scoped>
.mtm-sheet {
  padding: var(--space-2) var(--space-3) var(--space-4);
  display: flex;
  flex-direction: column;
}

.mtm-sheet__header {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  min-height: 40px;
  padding: 0 var(--space-1) var(--space-2);
}

.mtm-sheet__title {
  text-align: center;
  font-size: var(--fs-14);
  font-weight: var(--fw-semibold);
  color: var(--color-text);
  min-width: 0;
}

.mtm-sheet__back {
  justify-self: start;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: none;
  background: transparent;
  color: var(--color-text-soft);
  font-size: var(--fs-13);
  padding: 6px 8px;
  border-radius: var(--radius-2);
}

.mtm-sheet__header-spacer {
  justify-self: end;
  width: 52px;
}
</style>
