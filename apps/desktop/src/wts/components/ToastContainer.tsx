import { useToastStore } from '../stores/toastStore';

/**
 * 토스트 컨테이너 컴포넌트
 * 화면 우측 상단에 토스트 메시지 표시
 */
export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div
      data-testid="toast-container"
      className="fixed top-4 right-4 z-[60] flex flex-col gap-2 pointer-events-none"
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          data-testid={`toast-${toast.type}`}
          className={`
            pointer-events-auto
            px-4 py-3 rounded-lg shadow-lg
            text-sm font-medium
            animate-in slide-in-from-right-full fade-in
            duration-200
            ${toast.type === 'success' ? 'bg-green-600 text-white' : ''}
            ${toast.type === 'error' ? 'bg-red-600 text-white' : ''}
            ${toast.type === 'info' ? 'bg-blue-600 text-white' : ''}
          `}
          onClick={() => removeToast(toast.id)}
          role="alert"
        >
          {toast.message}
        </div>
      ))}
    </div>
  );
}
