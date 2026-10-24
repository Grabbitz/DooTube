// YouTube Music's player bar, bridged for DooTube's Now Playing card, Controls menu, and the
// system's media controls. Every action presses the site's own player-bar button.
(() => {
  if (location.hostname !== "music.youtube.com") return;
  const post = (message) => window.webkit.messageHandlers.__DOOTUBE_HANDLER__.postMessage(JSON.stringify(message));
  const buttons = {
    playpause: "ytmusic-player-bar #play-pause-button",
    next: "ytmusic-player-bar .next-button",
    previous: "ytmusic-player-bar .previous-button",
  };
  const press = (action) => {
    const button = buttons[action] && document.querySelector(buttons[action]);
    if (!button || button.disabled || button.hasAttribute("disabled")) return false;
    button.click();
    return true;
  };
  window.__dootubeControl = press;

  let last = "";
  const report = () => {
    const meta = navigator.mediaSession && navigator.mediaSession.metadata;
    const media = document.querySelector("ytmusic-player video, video");
    const artwork = meta && meta.artwork && meta.artwork.length ? meta.artwork[meta.artwork.length - 1].src : "";
    const state = {
      type: "nowplaying",
      title: (meta && meta.title) || "",
      artist: (meta && meta.artist) || "",
      artwork,
      playing: !!media && !media.paused && !media.ended,
    };
    const key = JSON.stringify(state);
    if (key !== last) {
      last = key;
      post(state);
    }
  };
  for (const name of ["play", "pause", "playing", "ended", "emptied", "loadedmetadata"]) {
    document.addEventListener(name, report, true);
  }
  if (window.MediaSession) {
    const metadata = Object.getOwnPropertyDescriptor(MediaSession.prototype, "metadata");
    if (metadata && metadata.set) {
      Object.defineProperty(MediaSession.prototype, "metadata", {
        configurable: true,
        get: metadata.get,
        set(value) {
          metadata.set.call(this, value);
          queueMicrotask(report);
        },
      });
    }
    // YouTube Music registers no next/previous handlers here, so the system's media keys and
    // Now Playing controls could only play and pause. Lend them the player bar's buttons
    // whenever the site has no handler of its own.
    const session = navigator.mediaSession;
    const setHandler = session.setActionHandler.bind(session);
    const fallbacks = {
      nexttrack: () => press("next"),
      previoustrack: () => press("previous"),
    };
    session.setActionHandler = (action, handler) => setHandler(action, handler || fallbacks[action] || null);
    for (const [action, handler] of Object.entries(fallbacks)) {
      try { setHandler(action, handler); } catch (_) {}
    }
  }
})();
