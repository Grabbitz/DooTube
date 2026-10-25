const samples = [
  { id: 'watch', title: 'Me at the zoo', kind: 'YouTube', artist: 'jawed', image: './assets/watch-poster.jpg' },
  { id: 'music', title: 'São Paulo', kind: 'Music', artist: 'The Weeknd & Anitta', image: './assets/music-stage.webp' },
];
const queuePanel = document.querySelector('#queue-panel');
const queueItems = document.querySelector('#queue-items');
const queueToggle = document.querySelector('#queue-toggle');
const queueStatus = document.querySelector('#queue-status');
const undoButton = document.querySelector('#undo-remove');
const dialog = document.querySelector('#search-dialog');
const searchInput = document.querySelector('#preview-search');
const results = document.querySelector('#search-results');
const playerDialog = document.querySelector('#player-dialog');
let view = new URLSearchParams(location.search).get('view') === 'music' ? 'music' : 'watch';
let queue = ['music'];
let removed = null;
let searchMatches = [];
let activeResult = 0;
document.querySelectorAll('svg.icon').forEach(svg => svg.setAttribute('aria-hidden', 'true'));
searchInput.setAttribute('role', 'combobox');
searchInput.setAttribute('aria-autocomplete', 'list');
searchInput.setAttribute('aria-controls', 'search-results');
searchInput.setAttribute('aria-expanded', 'false');
results.setAttribute('role', 'listbox');
results.setAttribute('aria-label', 'รายการตัวอย่าง');
dialog.addEventListener('close', () => searchInput.setAttribute('aria-expanded', String(dialog.open)));

function icon(name) {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('class', 'icon');
  svg.setAttribute('aria-hidden', 'true');
  const use = document.createElementNS(svg.namespaceURI, 'use');
  use.setAttribute('href', `#i-${name}`);
  svg.append(use);
  return svg;
}
function saveButtons() {
  const saved = queue.includes(view);
  document.querySelectorAll('[data-save]').forEach(button => {
    button.dataset.state = saved ? 'success' : 'default';
    button.setAttribute('aria-pressed', String(saved));
    button.replaceChildren(icon(saved ? 'check' : 'plus'));
    const label = document.createElement('span');
    label.textContent = saved ? 'อยู่ในคิวแล้ว' : view === 'watch' ? 'บันทึกไว้ดู' : 'บันทึกไว้ฟัง';
    button.append(label);
  });
}
function renderQueue() {
  queueItems.replaceChildren();
  document.querySelector('#queue-count').textContent = `${queue.length} รายการ`;
  document.querySelector('#header-queue-count').textContent = queue.length;
  if (!queue.length) {
    const empty = document.createElement('div');
    empty.className = 'queue-empty';
    const title = document.createElement('p');
    title.textContent = 'ยังไม่มีรายการที่บันทึก';
    const description = document.createElement('p');
    description.className = 'muted';
    description.textContent = 'ใช้ปุ่มบันทึกบนหน้าจอ เพื่อเก็บรายการไว้เปิดทีหลัง';
    empty.append(icon('list'), title, description);
    queueItems.append(empty);
  }
  for (const id of queue) {
    const item = samples.find(sample => sample.id === id);
    const row = document.createElement('div');
    row.className = 'queue-row';
    row.dataset.current = String(id === view);
    const open = document.createElement('button');
    open.className = 'queue-open';
    open.setAttribute('aria-label', `เปิดพรีวิว ${item.title}`);
    open.setAttribute('aria-current', id === view ? 'true' : 'false');
    const image = document.createElement('img');
    image.src = item.image;
    image.alt = '';
    image.width = 72;
    image.height = 52;
    const copy = document.createElement('span');
    copy.className = 'queue-copy';
    const title = document.createElement('span');
    title.className = 'queue-title';
    title.textContent = item.title;
    const kind = document.createElement('span');
    kind.className = 'queue-kind';
    kind.append(icon(id === 'watch' ? 'video' : 'music'), document.createTextNode(item.kind));
    copy.append(title, kind);
    open.append(image, copy);
    open.addEventListener('click', () => { queuePanel.close(); setView(id, true); window.scrollTo({ top: 0, behavior: 'instant' }); });
    const remove = document.createElement('button');
    remove.className = 'icon-button';
    remove.setAttribute('aria-label', `ลบ ${item.title} จากคิวตัวอย่าง`);
    remove.append(icon('close'));
    remove.addEventListener('click', () => {
      removed = { id, index: queue.indexOf(id) };
      queue = queue.filter(current => current !== id);
      queueStatus.textContent = `ลบ ${item.title} แล้ว`;
      undoButton.hidden = false;
      renderQueue();
      undoButton.focus({ preventScroll: true });
    });
    row.append(open, remove);
    queueItems.append(row);
  }
  saveButtons();
}
function setView(next, focus = false) {
  view = next;
  for (const mode of ['watch', 'music']) {
    const selected = view === mode;
    document.querySelector(`#${mode}-panel`).hidden = !selected;
    const tab = document.querySelector(`#${mode}-tab`);
    tab.setAttribute('aria-selected', String(selected));
    tab.tabIndex = selected ? 0 : -1;
  }
  document.querySelectorAll('.shelf-card').forEach(card => {
    const current = card.dataset.view === view;
    card.setAttribute('aria-current', String(current));
    card.querySelector('.card-open use').setAttribute('href', current ? '#i-check' : '#i-chevron');
  });
  document.title = `DooTube — ${view === 'watch' ? 'Watch' : 'Music'} concept`;
  renderQueue();
  if (focus) document.querySelector(`#${view}-tab`).focus({ preventScroll: true });
}
document.querySelectorAll('[data-view]').forEach(button => button.addEventListener('click', () => {
  setView(button.dataset.view);
  if (button.classList.contains('shelf-card')) {
    window.scrollTo({ top: 0, behavior: 'instant' });
    document.querySelector(`#${view}-tab`).focus({ preventScroll: true });
  }
}));
document.querySelector('.mode-nav').addEventListener('keydown', event => {
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) return;
  event.preventDefault();
  setView(event.key === 'Home' ? 'watch' : event.key === 'End' ? 'music' : view === 'watch' ? 'music' : 'watch', true);
});
document.querySelectorAll('[data-save]').forEach(button => button.addEventListener('click', () => {
  if (queue.includes(view)) return;
  queue.push(view);
  queueStatus.textContent = 'คิวตัวอย่าง ไม่ใช่คิวในแอป';
  renderQueue();
}));
undoButton.addEventListener('click', () => {
  if (removed && !queue.includes(removed.id)) queue.splice(Math.min(removed.index, queue.length), 0, removed.id);
  removed = null;
  undoButton.hidden = true;
  queueStatus.textContent = 'คิวตัวอย่าง ไม่ใช่คิวในแอป';
  renderQueue();
  document.querySelector('[role=tab][aria-selected=true]').focus({ preventScroll: true });
});
document.querySelector('#reset-queue').addEventListener('click', () => {
  queue = ['music'];
  removed = null;
  undoButton.hidden = true;
  queueStatus.textContent = 'คิวตัวอย่าง ไม่ใช่คิวในแอป';
  renderQueue();
});
queueToggle.addEventListener('click', () => {
  queuePanel.showModal();
  queueToggle.setAttribute('aria-expanded', 'true');
  (queuePanel.querySelector('.queue-open') || document.querySelector('#reset-queue')).focus({ preventScroll: true });
});
document.querySelector('#queue-close').addEventListener('click', () => queuePanel.close());
queuePanel.addEventListener('close', () => queueToggle.setAttribute('aria-expanded', String(queuePanel.open)));
document.querySelectorAll('[data-player]').forEach(button => button.addEventListener('click', () => {
  const capture = document.querySelector('#player-capture');
  const music = view === 'music';
  capture.src = music ? './assets/music-stage.webp' : './assets/watch-player.webp';
  capture.width = music ? 1000 : 1064;
  capture.height = music ? 501 : 800;
  capture.alt = music ? 'ภาพอ้างอิงสื่อ Music: แสงเหนือเวที São Paulo — ไม่ใช่ภาพ player เต็มหน้า' : 'ภาพนิ่ง player ทางการของ YouTube บนหน้า Me at the zoo';
  document.querySelector('#player-title').textContent = music ? 'ภาพอ้างอิง · Music media reference' : 'ภาพ player ทางการ';
  playerDialog.querySelector('.icon-button').setAttribute('aria-label', music ? 'ปิดภาพอ้างอิง Music' : 'ปิดภาพ player');
  playerDialog.showModal();
}));
function renderSearch() {
  const query = searchInput.value.trim().toLocaleLowerCase();
  searchMatches = samples.filter(item => `${item.title} ${item.artist} ${item.kind}`.toLocaleLowerCase().includes(query));
  activeResult = Math.min(activeResult, Math.max(0, searchMatches.length - 1));
  results.replaceChildren();
  if (!searchMatches.length) {
    const message = document.createElement('p');
    message.className = 'muted';
    message.textContent = 'ไม่พบในตัวอย่าง ลองค้นหา Me at the zoo หรือ São Paulo';
    results.append(message);
  }
  searchMatches.forEach((item, index) => {
    const button = document.createElement('button');
    button.className = 'search-result';
    button.dataset.active = String(index === activeResult);
    button.id = `search-result-${item.id}`;
    button.setAttribute('role', 'option');
    button.setAttribute('aria-selected', String(index === activeResult));
    button.tabIndex = -1;
    const title = document.createElement('span');
    title.textContent = item.title;
    const kind = document.createElement('span');
    kind.className = 'result-kind';
    kind.textContent = item.kind;
    button.append(title, kind);
    button.addEventListener('click', () => { dialog.close(); setView(item.id, true); });
    results.append(button);
  });
  if (searchMatches.length) searchInput.setAttribute('aria-activedescendant', `search-result-${searchMatches[activeResult].id}`);
  else searchInput.removeAttribute('aria-activedescendant');
}
function openSearch() {
  if (dialog.open) { dialog.close(); return; }
  if (queuePanel.open) queuePanel.close();
  if (playerDialog.open) playerDialog.close();
  searchInput.value = '';
  activeResult = 0;
  renderSearch();
  dialog.showModal();
  searchInput.setAttribute('aria-expanded', 'true');
  searchInput.focus({ preventScroll: true });
}
document.querySelector('#search-trigger').addEventListener('click', openSearch);
searchInput.addEventListener('input', () => { activeResult = 0; renderSearch(); });
searchInput.addEventListener('keydown', event => {
  if (['ArrowDown', 'ArrowUp'].includes(event.key) && searchMatches.length) {
    event.preventDefault();
    activeResult = (activeResult + (event.key === 'ArrowDown' ? 1 : -1) + searchMatches.length) % searchMatches.length;
    renderSearch();
  } else if (event.key === 'Enter' && searchMatches.length) {
    event.preventDefault();
    const selected = searchMatches[activeResult];
    dialog.close();
    setView(selected.id, true);
  }
});
for (const surface of [dialog, queuePanel, playerDialog]) {
  surface.addEventListener('click', event => {
    if (event.target !== surface) return;
    const bounds = surface.getBoundingClientRect();
    if (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom) surface.close();
  });
}
document.addEventListener('keydown', event => {
  if (event.key === 'Escape' && dialog.open) {
    event.preventDefault();
    dialog.close();
    return;
  }
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
    event.preventDefault();
    openSearch();
  }
}, true);
setView(view);
