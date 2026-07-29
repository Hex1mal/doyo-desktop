const state = $state({
  isOpen: false,
  query: '',
  results: [] as any[],
});

export const searchStore = {
  get isOpen() {
    return state.isOpen;
  },
  get query() {
    return state.query;
  },
  get results() {
    return state.results;
  },

  open() {
    state.isOpen = true;
  },
  close() {
    state.isOpen = false;
    state.query = '';
    state.results = [];
  },
  setQuery(q: string) {
    state.query = q;
  },
  setResults(r: any[]) {
    state.results = r;
  },
};
