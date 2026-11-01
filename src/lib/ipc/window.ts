// Window controls for the custom title bar (docs/modules/full-window.md §5).
import { getCurrentWindow } from '@tauri-apps/api/window';

export const currentWindow = {
  minimize: () => getCurrentWindow().minimize(),
  toggleMaximize: () => getCurrentWindow().toggleMaximize(),
  close: () => getCurrentWindow().close(),
};

export const onWindowClose = (canClose: () => boolean) => getCurrentWindow().onCloseRequested(e => { if (!canClose()) e.preventDefault(); });
