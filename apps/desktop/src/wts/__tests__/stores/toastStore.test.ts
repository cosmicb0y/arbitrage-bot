import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { useToastStore } from '../../stores/toastStore';

describe('toastStore', () => {
  beforeEach(() => {
    useToastStore.setState({ toasts: [] });
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('showToast', () => {
    it('토스트가 추가되어야 한다', () => {
      useToastStore.getState().showToast('success', '테스트 메시지');

      expect(useToastStore.getState().toasts).toHaveLength(1);
      expect(useToastStore.getState().toasts[0].type).toBe('success');
      expect(useToastStore.getState().toasts[0].message).toBe('테스트 메시지');
    });

    it('error 타입 토스트가 추가되어야 한다', () => {
      useToastStore.getState().showToast('error', '에러 메시지');

      expect(useToastStore.getState().toasts[0].type).toBe('error');
    });

    it('info 타입 토스트가 추가되어야 한다', () => {
      useToastStore.getState().showToast('info', '정보 메시지');

      expect(useToastStore.getState().toasts[0].type).toBe('info');
    });

    it('3초 후 자동으로 제거되어야 한다', () => {
      useToastStore.getState().showToast('success', '자동 제거');

      expect(useToastStore.getState().toasts).toHaveLength(1);

      vi.advanceTimersByTime(3000);

      expect(useToastStore.getState().toasts).toHaveLength(0);
    });

    it('여러 토스트가 추가 가능해야 한다', () => {
      useToastStore.getState().showToast('success', '첫 번째');
      useToastStore.getState().showToast('error', '두 번째');

      expect(useToastStore.getState().toasts).toHaveLength(2);
    });
  });

  describe('removeToast', () => {
    it('특정 토스트가 제거되어야 한다', () => {
      useToastStore.getState().showToast('success', '메시지');
      const toastId = useToastStore.getState().toasts[0].id;

      useToastStore.getState().removeToast(toastId);

      expect(useToastStore.getState().toasts).toHaveLength(0);
    });
  });

  describe('clearToasts', () => {
    it('모든 토스트가 제거되어야 한다', () => {
      useToastStore.getState().showToast('success', '첫 번째');
      useToastStore.getState().showToast('error', '두 번째');

      useToastStore.getState().clearToasts();

      expect(useToastStore.getState().toasts).toHaveLength(0);
    });
  });
});
