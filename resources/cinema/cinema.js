(() => {
  'use strict';

  const $ = (selector) => document.querySelector(selector);
  const hero = $('#hero');
  const heroImage = $('#hero-image');
  const heroTitle = $('#hero-title');
  const heroByline = $('#hero-byline');
  const heroSource = $('#hero-source');
  const heroMark = $('#hero-mark');
  const heroContext = $('#hero-context');
  const heroNote = $('#hero-note');
  const heroOpen = $('#hero-open');
  const heroSave = $('#hero-save');
  const shelf = $('#shelf');
  const destinations = $('#destinations');
  const queueError = $('#queue-error');
  const queueOpen = $('#queue-open');
  const trackSection = $('#track-section');
  const trackImage = $('#track-image');
  const trackArt = $('.track-art');
  let state = null;
  let featuredAction = null;
  let shelfNodes = new Map();
  let destinationNodes = new Map();

  const text = (value) => typeof value === 'string' ? value.trim() : '';
  const modeName = (mode) => mode === 'music' ? 'YouTube Music' : 'YouTube';
  const icon = (id) => {
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.classList.add('icon');
    svg.setAttribute('aria-hidden', 'true');
    const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
    use.setAttribute('href', `#${id}`);
    svg.append(use);
    return svg;
  };
  const action = (payload) => {
    const handler = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.cinema;
    if (!handler) return;
    handler.postMessage(JSON.stringify(payload));
  };
  const openableUrl = (value) => {
    if (!text(value)) return null;
    try {
      const url = new URL(value);
      const hosts = new Set(['youtube.com', 'www.youtube.com', 'music.youtube.com', 'm.youtube.com', 'youtu.be', 'www.youtu.be']);
      if (url.protocol !== 'https:' || !hosts.has(url.hostname.toLowerCase()) || url.username || url.password || url.port) return null;
      return url.href;
    } catch (_) {
      return null;
    }
  };
  const imageUrl = (value) => {
    if (!text(value)) return null;
    try {
      const url = new URL(value);
      const host = url.hostname.toLowerCase();
      const hosts = ['ytimg.com', 'googleusercontent.com', 'ggpht.com'];
      if (url.protocol !== 'https:' || url.username || url.password || url.port) return null;
      if (!hosts.some((domain) => host === domain || host.endsWith(`.${domain}`))) return null;
      return url.href;
    } catch (_) {
      return null;
    }
  };
  const imageState = (image, container, value) => {
    const url = imageUrl(value);
    image.onerror = () => container.classList.remove('has-image');
    image.onload = () => {
      if (imageUrl(image.currentSrc || image.src) === url) container.classList.add('has-image');
    };
    if (!url) {
      image.removeAttribute('src');
      container.classList.remove('has-image');
      return;
    }
    if (image.getAttribute('src') !== url) {
      container.classList.remove('has-image');
      image.src = url;
    }
    if (image.complete && image.naturalWidth > 0) container.classList.add('has-image');
    else if (image.complete) container.classList.remove('has-image');
  };
  const makeSnapshot = (snapshot) => {
    if (!snapshot || typeof snapshot !== 'object' || Array.isArray(snapshot)) return null;
    const mode = snapshot.mode === 'music' ? 'music' : snapshot.mode === 'youtube' ? 'youtube' : null;
    if (!mode) return null;
    const queue = Array.isArray(snapshot.queue) ? snapshot.queue.filter((item) => item && typeof item === 'object').map((item) => ({
      url: openableUrl(item.url),
      resume_url: openableUrl(item.resume_url),
      title: text(item.title),
      thumbnail: imageUrl(item.thumbnail),
      mode: item.mode === 'music' ? 'music' : 'youtube',
      position: Number.isFinite(item.position) ? item.position : null,
    })).filter((item) => item.url || item.resume_url) : [];
    const currentData = snapshot.current && typeof snapshot.current === 'object' ? snapshot.current : null;
    const currentUrl = currentData ? openableUrl(currentData.url) : null;
    const current = currentData && currentUrl ? {
      url: currentUrl,
      title: text(currentData.title),
      thumbnail: imageUrl(currentData.thumbnail),
      saved: currentData.saved === true,
    } : null;
    const trackData = snapshot.track && typeof snapshot.track === 'object' ? snapshot.track : {};
    const track = { title: text(trackData.title), artist: text(trackData.artist), artwork: imageUrl(trackData.artwork), playing: trackData.playing === true };
    const destinationList = Array.isArray(snapshot.destinations) ? snapshot.destinations.filter((item) => item && typeof item === 'object').map((item) => ({ title: text(item.title), url: openableUrl(item.url) })).filter((item) => item.title && item.url) : [];
    return { mode, queue, queue_error: text(snapshot.queue_error), current, track, destinations: destinationList };
  };
  const makeShelfCard = () => {
    const button = document.createElement('button');
    button.className = 'shelf-card';
    button.type = 'button';
    const art = document.createElement('span');
    art.className = 'card-art';
    const img = document.createElement('img');
    img.alt = '';
    img.loading = 'lazy';
    const fallback = document.createElement('span');
    fallback.className = 'fallback-title';
    const tag = document.createElement('span');
    tag.className = 'card-tag';
    const play = document.createElement('span');
    play.className = 'card-play';
    play.append(icon('i-play'));
    art.append(img, fallback, tag, play);
    const copy = document.createElement('span');
    copy.className = 'card-copy';
    const title = document.createElement('span');
    title.className = 'card-title';
    const subtitle = document.createElement('span');
    subtitle.className = 'card-subtitle';
    const mode = document.createElement('span');
    mode.className = 'card-mode';
    copy.append(title, subtitle, mode);
    button.append(art, copy);
    button._parts = { art, img, fallback, tag, title, subtitle, mode };
    return button;
  };
  const makeDestinationCard = () => {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'destination-card';
    const label = document.createElement('span');
    const arrow = icon('i-arrow');
    button.append(label, arrow);
    button._label = label;
    return button;
  };
  const syncNodes = (container, nextNodes) => {
    const focusKey = document.activeElement && container.contains(document.activeElement) ? document.activeElement.dataset.focusKey : null;
    const existing = new Set(nextNodes);
    for (const node of Array.from(container.children)) if (!existing.has(node)) node.remove();
    nextNodes.forEach((node, index) => {
      const at = container.children[index];
      if (at !== node) container.insertBefore(node, at || null);
    });
    if (focusKey) {
      const replacement = Array.from(container.querySelectorAll('[data-focus-key]')).find((node) => node.dataset.focusKey === focusKey);
      if (replacement && replacement !== document.activeElement) replacement.focus({ preventScroll: true });
    }
  };
  const renderShelf = () => {
    const items = state.queue;
    const next = [];
    const keep = new Set();
    const occurrences = new Map();
    for (const [index, item] of items.entries()) {
      const identity = JSON.stringify([item.mode, item.url]);
      const occurrence = occurrences.get(identity) || 0;
      occurrences.set(identity, occurrence + 1);
      const key = `${identity}:${occurrence}`;
      keep.add(key);
      let card = shelfNodes.get(key);
      if (!card) card = makeShelfCard();
      shelfNodes.set(key, card);
      card.dataset.focusKey = `saved:${key}`;
      card.dataset.openIndex = String(index);
      card.setAttribute('aria-label', `Open ${item.title || 'saved item'} on ${modeName(item.mode)}`);
      const parts = card._parts;
      parts.title.textContent = item.title || 'Untitled saved item';
      parts.subtitle.textContent = item.position === null ? '' : `Saved at ${Math.floor(item.position / 60)}:${String(item.position % 60).padStart(2, '0')}`;
      parts.subtitle.hidden = item.position === null;
      parts.mode.textContent = modeName(item.mode);
      parts.tag.textContent = modeName(item.mode);
      parts.fallback.textContent = item.title || modeName(item.mode);
      imageState(parts.img, parts.art, item.thumbnail);
      next.push(card);
    }
    for (const key of shelfNodes.keys()) if (!keep.has(key)) shelfNodes.delete(key);
    if (items.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'shelf-empty';
      const copy = document.createElement('div');
      copy.className = 'empty-copy';
      const title = document.createElement('h3');
      title.textContent = state.queue_error ? 'Up Next unavailable' : 'Nothing saved yet';
      const message = document.createElement('p');
      message.textContent = state.queue_error ? 'Your saved queue is untouched.' : 'Save a video with ⌘S while watching. It stays here until you remove it.';
      copy.append(title, message);
      empty.append(copy);
      next.push(empty);
    }
    syncNodes(shelf, next);
  };
  const renderDestinations = () => {
    const next = [];
    const keep = new Set();
    for (const [index, destination] of state.destinations.entries()) {
      const key = JSON.stringify([destination.url, destination.title, index]);
      keep.add(key);
      let card = destinationNodes.get(key);
      if (!card) card = makeDestinationCard();
      destinationNodes.set(key, card);
      card.dataset.focusKey = `destination:${key}`;
      card.dataset.destinationIndex = String(index);
      card._label.textContent = destination.title;
      card.setAttribute('aria-label', `Browse ${destination.title}`);
      next.push(card);
    }
    for (const key of destinationNodes.keys()) if (!keep.has(key)) destinationNodes.delete(key);
    if (next.length === 0) {
      const empty = document.createElement('p');
      empty.className = 'empty-copy';
      empty.textContent = 'No browse destinations are available right now.';
      next.push(empty);
    }
    syncNodes(destinations, next);
  };
  const setHero = (feature, kind) => {
    const music = state.mode === 'music';
    heroSource.textContent = modeName(state.mode);
    heroMark.replaceChildren(icon(music ? 'i-music' : 'i-video'));
    hero.classList.toggle('is-empty', !feature);
    heroTitle.textContent = feature ? (feature.title || 'Untitled') : (music ? 'Your next listen.' : 'Your next watch.');
    hero.classList.toggle('long-title', heroTitle.textContent.length > 50);
    hero.classList.toggle('very-long-title', heroTitle.textContent.length > 90);
    heroByline.textContent = feature ? (feature.artist || '') : (music ? 'Search songs above, or open YouTube Music.' : 'Search above, or browse YouTube to find a video.');
    heroByline.hidden = feature && !feature.artist;
    heroContext.textContent = feature && kind === 'saved' ? 'From Up Next' : feature && kind === 'track' ? 'Now playing' : '';
    heroContext.hidden = !heroContext.textContent;
    imageState(heroImage, $('.hero-art'), feature && feature.thumbnail);
    featuredAction = feature ? kind === 'saved' ? { action: 'open', url: feature.resume_url || feature.url } : { action: 'resume' } : null;
    heroOpen.disabled = !featuredAction;
    heroOpen.querySelector('span').textContent = feature ? (kind === 'saved' ? 'Play saved item' : 'Return to player') : (music ? 'Browse Music' : 'Browse YouTube');
    if (!feature && state.destinations.length) {
      featuredAction = { action: 'open', url: state.destinations[0].url };
      heroOpen.disabled = false;
    }
    heroSave.hidden = !state.current || state.current.saved;
    heroSave.disabled = Boolean(state.queue_error);
    heroNote.textContent = state.current && state.current.saved ? 'This title is already saved to Up Next.' : '';
  };
  const renderTrack = () => {
    const track = state.track;
    const visible = state.mode === 'youtube' && !!track.title;
    trackSection.hidden = !visible;
    if (!visible) return;
    $('#track-title').textContent = track.title;
    $('#track-artist').textContent = track.artist;
    $('#track-artist').hidden = !track.artist;
    $('#track-state').textContent = track.playing ? 'Playing' : 'Not playing';
    imageState(trackImage, trackArt, track.artwork);
  };
  const render = (incoming) => {
    const next = makeSnapshot(incoming);
    if (!next) return;
    state = next;
    const current = state.current;
    const saved = state.queue.find((item) => item.mode === state.mode);
    const track = state.mode === 'music' && state.track.title ? { title: state.track.title, artist: state.track.artist, thumbnail: state.track.artwork } : null;
    if (current) setHero({ title: state.mode === 'music' && track ? track.title : current.title, thumbnail: state.mode === 'music' && track && track.thumbnail || current.thumbnail, artist: state.mode === 'music' && track ? track.artist : '' }, 'current');
    else if (saved) setHero({ title: saved.title, thumbnail: saved.thumbnail, artist: '' , url: saved.url, resume_url: saved.resume_url }, 'saved');
    else if (track) setHero(track, 'track');
    else setHero(null, null);
    renderShelf();
    renderDestinations();
    $('#destinations-title').textContent = `Browse ${modeName(state.mode)}`;
    renderTrack();
    queueError.hidden = !state.queue_error;
    queueError.textContent = state.queue_error ? `Up Next could not be loaded: ${state.queue_error}` : '';
    queueOpen.disabled = false;
  };

  heroOpen.addEventListener('click', () => {
    if (featuredAction) action(featuredAction);
  });
  heroSave.addEventListener('click', () => action({ action: 'save' }));
  queueOpen.addEventListener('click', () => action({ action: 'queue' }));
  shelf.addEventListener('click', (event) => {
    const button = event.target.closest('button[data-open-index]');
    if (!button || !state) return;
    const item = state.queue[Number(button.dataset.openIndex)];
    const url = item && (item.resume_url || item.url);
    if (url) action({ action: 'open', url });
  });
  destinations.addEventListener('click', (event) => {
    const button = event.target.closest('button[data-destination-index]');
    if (!button || !state) return;
    const item = state.destinations[Number(button.dataset.destinationIndex)];
    if (item) action({ action: 'open', url: item.url });
  });

  Object.defineProperty(window, 'dootube', { value: Object.freeze({ render }), configurable: false, writable: false });
  action({ action: 'ready' });
})();
