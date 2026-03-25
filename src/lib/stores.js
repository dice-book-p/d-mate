import { writable, get } from "svelte/store";

export const currentPage = writable("dashboard");
export const settings = writable({});
export const dashboardData = writable(null);
export const loading = writable(false);
export const alerts = writable([]);
export const toastMessage = writable(null);
export const pageDirty = writable(false);

export function showToast(message, type = "info") {
  toastMessage.set({ message, type });
  setTimeout(() => toastMessage.set(null), 3000);
}

/**
 * 페이지 이동 시 미저장 확인
 * @param {string} targetPage
 * @returns {boolean} 이동 허용 여부
 */
export function navigateTo(targetPage) {
  if (get(pageDirty)) {
    const ok = confirm("저장하지 않은 변경사항이 있습니다. 이동하시겠습니까?");
    if (!ok) return false;
  }
  pageDirty.set(false);
  currentPage.set(targetPage);
  return true;
}
