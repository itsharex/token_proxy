import { useI18n } from "@/lib/i18n";

/**
 * 分离的语言订阅者组件
 * 
 * 问题：原先在 App.tsx 中调用 useI18n() 会导致整个应用树重渲染
 * 解决：将订阅行为隔离到这个独立的组件
 * 
 * 当语言改变时，只有 LanguageObserver 组件重渲染，避免全局重渲染
 */
export function LanguageObserver() {
  useI18n(); // 订阅语言状态变化

  // 不需要渲染任何内容，只是为了订阅
  return null;
}
