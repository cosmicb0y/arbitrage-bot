import { create } from 'zustand';

export type ToastType = 'success' | 'error' | 'info';

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
}

export interface ToastState {
  toasts: Toast[];
  showToast: (type: ToastType, message: string) => void;
  removeToast: (id: string) => void;
  clearToasts: () => void;
}

/** 토스트 자동 제거 시간 (ms) */
const TOAST_DURATION = 3000;

/**
 * 토스트 스토어
 * 간단한 알림 메시지 관리
 */
export const useToastStore = create<ToastState>()((set) => ({
  toasts: [],

  showToast: (type: ToastType, message: string) => {
    const id =
      globalThis.crypto?.randomUUID?.() ??
      `${Date.now()}-${Math.random().toString(16).slice(2)}`;

    const newToast: Toast = { id, type, message };

    set((state) => ({
      toasts: [...state.toasts, newToast],
    }));

    // 자동 제거
    setTimeout(() => {
      set((state) => ({
        toasts: state.toasts.filter((t) => t.id !== id),
      }));
    }, TOAST_DURATION);
  },

  removeToast: (id: string) =>
    set((state) => ({
      toasts: state.toasts.filter((t) => t.id !== id),
    })),

  clearToasts: () => set({ toasts: [] }),
}));
