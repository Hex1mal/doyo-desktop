export type ZoomAction = 'in' | 'out' | 'reset';

export const ZOOM_MIN = 0.8;
export const ZOOM_MAX = 2;
export const ZOOM_STEP = 0.1;

export function clampZoom(level: number) {
  return Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, Number(level.toFixed(2))));
}

export function nextZoom(level: number, action: ZoomAction) {
  if (action === 'reset') return 1;
  return clampZoom(level + (action === 'in' ? ZOOM_STEP : -ZOOM_STEP));
}

export function zoomActionFromKeyboard(event: {
  key: string;
  code: string;
  ctrlKey: boolean;
  metaKey: boolean;
}): ZoomAction | null {
  if (!event.ctrlKey && !event.metaKey) return null;
  const key = event.key.toLowerCase();
  if (key === '+' || key === '=' || event.code === 'NumpadAdd') return 'in';
  if (key === '-' || event.code === 'NumpadSubtract') return 'out';
  if (key === '0' || event.code === 'Numpad0') return 'reset';
  return null;
}
