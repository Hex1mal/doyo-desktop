const state = $state({
  isOpen: false,
  query: '',
  selectedIndex: 0,
});

export const commandPaletteStore = {
  get isOpen() {
    return state.isOpen;
  },
  get query() {
    return state.query;
  },
  get selectedIndex() {
    return state.selectedIndex;
  },

  open() {
    state.isOpen = true;
    state.query = '';
    state.selectedIndex = 0;
  },

  close() {
    state.isOpen = false;
    state.query = '';
  },

  setQuery(q: string) {
    state.query = q;
    state.selectedIndex = 0;
  },

  moveUp() {
    state.selectedIndex = Math.max(0, state.selectedIndex - 1);
  },

  moveDown(max: number) {
    state.selectedIndex = Math.min(max - 1, state.selectedIndex + 1);
  },
};
