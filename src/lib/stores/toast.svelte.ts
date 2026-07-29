export type ToastType = 'info' | 'success' | 'error';

export interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

const state = $state({
  items: [] as Toast[],
  nextId: 1,
});

export const toast = {
  get items() {
    return state.items;
  },

  show(message: string, type: ToastType = 'info', ms = 2800) {
    const id = state.nextId++;
    state.items = [...state.items, { id, message, type }];
    setTimeout(() => {
      state.items = state.items.filter((t) => t.id !== id);
    }, ms);
  },

  info(message: string) {
    this.show(message, 'info');
  },
  success(message: string) {
    this.show(message, 'success');
  },
  error(message: string) {
    this.show(message, 'error', 4000);
  },

  dismiss(id: number) {
    state.items = state.items.filter((t) => t.id !== id);
  },
};
