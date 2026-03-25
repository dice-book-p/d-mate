import { writable } from "svelte/store";

export const dialogState = writable(null);

/**
 * 확인 다이얼로그 표시
 * @param {object} opts - { type, title, message, confirmText, onConfirm }
 */
export function showDialog(opts) {
  dialogState.set(opts);
}

export function closeDialog() {
  dialogState.set(null);
}
