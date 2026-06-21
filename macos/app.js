const STORAGE_KEYS = {
    favorites: 'animai.macos.favorites.v1',
    theme: 'animai.macos.theme.v1',
};

const state = {
    positiveTags: new Set(),
    negativeTags: new Set(),
    tagElements: new Map(),
};

function storageGet(key, fallback) {
    try {
        const value = localStorage.getItem(key);
        return value ? JSON.parse(value) : fallback;
    } catch {
        return fallback;
    }
}

function storageSet(key, value) {
    localStorage.setItem(key, JSON.stringify(value));
}

function getPromptsData() {
    const data = window.ANIMAI_PROMPTS;
    if (!data || !Array.isArray(data.sfw) || !Array.isArray(data.nsfw) || !Array.isArray(data.negative)) {
        return { sfw: [], nsfw: [], negative: [] };
    }
    return data;
}

function getFavorites() {
    const favorites = storageGet(STORAGE_KEYS.favorites, []);
    return Array.isArray(favorites)
        ? favorites.filter(item => item && typeof item.name === 'string')
        : [];
}

function setFavorites(favorites) {
    const sorted = [...favorites].sort((a, b) => a.name.localeCompare(b.name, 'ja'));
    storageSet(STORAGE_KEYS.favorites, sorted);
    renderFavorites(sorted);
}

function renderFavorites(presets) {
    const listEl = document.getElementById('favorites-list');
    if (!listEl) return;

    listEl.innerHTML = '';

    if (presets.length === 0) {
        listEl.innerHTML = '<div class="no-favorites">保存されたお気に入りはまだありません。</div>';
        return;
    }

    presets.forEach((preset) => {
        const item = document.createElement('div');
        item.className = 'favorite-item';
        item.addEventListener('click', () => loadPreset(preset));

        const content = document.createElement('div');
        content.className = 'favorite-item-content';

        const nameEl = document.createElement('div');
        nameEl.className = 'favorite-item-name';
        nameEl.textContent = `★ ${preset.name}`;

        const metaEl = document.createElement('div');
        metaEl.className = 'favorite-item-meta';
        const posCount = Array.isArray(preset.positive) ? preset.positive.length : 0;
        const negCount = Array.isArray(preset.negative) ? preset.negative.length : 0;
        metaEl.innerHTML = `<span>ポジティブ ${posCount}</span> <span>ネガティブ ${negCount}</span>`;

        const actions = document.createElement('div');
        actions.className = 'favorite-item-actions';

        const deleteBtn = document.createElement('button');
        deleteBtn.className = 'fav-delete-btn';
        deleteBtn.textContent = '削除';
        deleteBtn.title = '削除';
        deleteBtn.addEventListener('click', (event) => {
            event.stopPropagation();
            if (confirm(`お気に入り「${preset.name}」を削除しますか？`)) {
                handleDeleteFavorite(preset.name);
            }
        });

        content.appendChild(nameEl);
        content.appendChild(metaEl);
        actions.appendChild(deleteBtn);
        item.appendChild(content);
        item.appendChild(actions);
        listEl.appendChild(item);
    });
}

function loadPreset(preset) {
    state.positiveTags.clear();
    state.negativeTags.clear();
    document.querySelectorAll('.prompt-tag.active').forEach(btn => btn.classList.remove('active'));

    if (Array.isArray(preset.positive)) {
        preset.positive.forEach(tag => {
            state.positiveTags.add(tag);
            setTagActive(tag, false, true);
        });
    }

    if (Array.isArray(preset.negative)) {
        preset.negative.forEach(tag => {
            state.negativeTags.add(tag);
            setTagActive(tag, true, true);
        });
    }

    updateOutputs();
}

function handleSaveFavorite() {
    const inputEl = document.getElementById('favorite-name-input');
    const name = inputEl.value.trim();

    if (!name) {
        alert('お気に入り名を入力してください。');
        return;
    }

    if (state.positiveTags.size === 0 && state.negativeTags.size === 0) {
        alert('タグが選択されていません。タグを選択してから保存してください。');
        return;
    }

    const preset = {
        name,
        positive: Array.from(state.positiveTags),
        negative: Array.from(state.negativeTags),
    };

    const favorites = getFavorites().filter(item => item.name !== name);
    favorites.push(preset);
    setFavorites(favorites);
    inputEl.value = '';
}

function handleDeleteFavorite(name) {
    setFavorites(getFavorites().filter(item => item.name !== name));
}

function exportFavorites() {
    const payload = {
        app: 'AnimaI T2I',
        version: 1,
        exportedAt: new Date().toISOString(),
        favorites: getFavorites(),
    };
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = 'animai-favorites.json';
    link.click();
    URL.revokeObjectURL(url);
}

function normalizeImportedFavorites(json) {
    const source = Array.isArray(json) ? json : json.favorites;
    if (!Array.isArray(source)) {
        throw new Error('favorites array not found');
    }

    return source.map((item) => ({
        name: String(item.name || '').trim(),
        positive: Array.isArray(item.positive) ? item.positive.map(String) : [],
        negative: Array.isArray(item.negative) ? item.negative.map(String) : [],
    })).filter(item => item.name);
}

function importFavorites(file) {
    if (!file) return;

    const reader = new FileReader();
    reader.onload = () => {
        try {
            const imported = normalizeImportedFavorites(JSON.parse(reader.result));
            const merged = new Map(getFavorites().map(item => [item.name, item]));
            imported.forEach(item => merged.set(item.name, item));
            setFavorites(Array.from(merged.values()));
            alert(`${imported.length}件のお気に入りをインポートしました。`);
        } catch (error) {
            console.error(error);
            alert('お気に入りJSONを読み込めませんでした。');
        }
    };
    reader.readAsText(file);
}

function tagKey(promptText, isNegative) {
    return `${isNegative ? 'negative' : 'positive'}::${promptText}`;
}

function registerTagElement(promptText, isNegative, element) {
    const key = tagKey(promptText, isNegative);
    if (!state.tagElements.has(key)) {
        state.tagElements.set(key, new Set());
    }
    state.tagElements.get(key).add(element);
}

function setTagActive(promptText, isNegative, active) {
    const elements = state.tagElements.get(tagKey(promptText, isNegative));
    if (!elements) return;
    elements.forEach(element => element.classList.toggle('active', active));
}

function createTagElement(item, isNegative) {
    const btn = document.createElement('button');
    btn.className = 'prompt-tag';
    btn.type = 'button';

    const jpSpan = document.createElement('span');
    jpSpan.className = 'jp-name';
    jpSpan.textContent = item.japanese;

    const enSpan = document.createElement('span');
    enSpan.className = 'en-name';
    enSpan.textContent = item.prompt;

    btn.appendChild(jpSpan);
    btn.appendChild(enSpan);
    btn.addEventListener('click', (event) => {
        event.stopPropagation();
        toggleTag(item.prompt, isNegative);
    });

    registerTagElement(item.prompt, isNegative, btn);
    return btn;
}

function toggleTag(promptText, isNegative) {
    const targetSet = isNegative ? state.negativeTags : state.positiveTags;
    const nextActive = !targetSet.has(promptText);

    if (nextActive) {
        targetSet.add(promptText);
    } else {
        targetSet.delete(promptText);
    }

    setTagActive(promptText, isNegative, nextActive);
    updateOutputs();
}

function updateOutputs() {
    document.getElementById('positive-prompt-output').value = Array.from(state.positiveTags).join(', ');
    document.getElementById('negative-prompt-output').value = Array.from(state.negativeTags).join(', ');
}

function renderCategories(categories, containerEl, isNegative, defaultOpenFirst = false) {
    containerEl.innerHTML = '';

    if (!categories.length) {
        containerEl.innerHTML = '<div class="loading">データがありません。</div>';
        return;
    }

    categories.forEach((cat, index) => {
        const card = document.createElement('div');
        const isCollapsed = !(defaultOpenFirst && index === 0);
        card.className = `category-card${isCollapsed ? ' collapsed' : ''}`;

        const title = document.createElement('div');
        title.className = 'category-title';
        title.textContent = cat.name;
        title.addEventListener('click', () => card.classList.toggle('collapsed'));

        const body = document.createElement('div');
        body.className = 'category-body';

        const inner = document.createElement('div');
        inner.className = 'category-body-inner';
        cat.tags.forEach(item => inner.appendChild(createTagElement(item, isNegative)));

        body.appendChild(inner);
        card.appendChild(title);
        card.appendChild(body);
        containerEl.appendChild(card);
    });
}

function handleManualAdd(inputEl, isNegative) {
    const text = inputEl.value.trim();
    if (!text) return;

    text.split(',')
        .map(tag => tag.trim())
        .filter(Boolean)
        .forEach((tag) => {
            const targetSet = isNegative ? state.negativeTags : state.positiveTags;
            targetSet.add(tag);
            setTagActive(tag, isNegative, true);
        });

    inputEl.value = '';
    updateOutputs();
}

async function copyText(btnEl, textValue) {
    if (!textValue) return;

    try {
        if (navigator.clipboard && window.isSecureContext) {
            await navigator.clipboard.writeText(textValue);
        } else {
            const temp = document.createElement('textarea');
            temp.value = textValue;
            temp.style.position = 'fixed';
            temp.style.left = '-9999px';
            document.body.appendChild(temp);
            temp.focus();
            temp.select();
            document.execCommand('copy');
            temp.remove();
        }
        showCopiedState(btnEl);
    } catch (error) {
        console.error('Could not copy text:', error);
        alert('コピーできませんでした。テキスト欄から手動でコピーしてください。');
    }
}

function showCopiedState(btnEl) {
    btnEl.classList.add('copied');
    const copyTextEl = btnEl.querySelector('.copy-text');
    const successTextEl = btnEl.querySelector('.copy-success-text');
    copyTextEl.style.display = 'none';
    successTextEl.style.display = 'inline';

    setTimeout(() => {
        btnEl.classList.remove('copied');
        copyTextEl.style.display = 'inline';
        successTextEl.style.display = 'none';
    }, 1600);
}

function setupTheme() {
    const themeToggleBtn = document.getElementById('theme-toggle-btn');
    const sunIcon = themeToggleBtn.querySelector('.theme-icon-sun');
    const moonIcon = themeToggleBtn.querySelector('.theme-icon-moon');

    function updateThemeUI(theme) {
        sunIcon.style.display = theme === 'light' ? 'none' : 'inline';
        moonIcon.style.display = theme === 'light' ? 'inline' : 'none';
    }

    const savedTheme = storageGet(STORAGE_KEYS.theme, 'dark');
    document.documentElement.setAttribute('data-theme', savedTheme);
    updateThemeUI(savedTheme);

    themeToggleBtn.addEventListener('click', () => {
        const currentTheme = document.documentElement.getAttribute('data-theme');
        const nextTheme = currentTheme === 'dark' ? 'light' : 'dark';
        document.documentElement.setAttribute('data-theme', nextTheme);
        storageSet(STORAGE_KEYS.theme, nextTheme);
        updateThemeUI(nextTheme);
    });
}

document.addEventListener('DOMContentLoaded', () => {
    const data = getPromptsData();
    renderCategories(data.sfw, document.getElementById('sfw-categories'), false, true);
    renderCategories(data.nsfw, document.getElementById('nsfw-categories'), false, false);
    renderCategories(data.negative, document.getElementById('negative-categories'), true, true);
    renderFavorites(getFavorites());

    document.querySelectorAll('.tab-btn').forEach((btn) => {
        btn.addEventListener('click', () => {
            const tabName = btn.getAttribute('data-tab');
            document.querySelectorAll('.tab-btn').forEach(tabBtn => tabBtn.classList.remove('active'));
            document.querySelectorAll('.tab-panel').forEach(panel => panel.classList.remove('active'));
            btn.classList.add('active');
            document.getElementById(`panel-${tabName}`).classList.add('active');
        });
    });

    const nsfwToggle = document.getElementById('nsfw-toggle');
    const nsfwTabBtn = document.getElementById('nsfw-tab-btn');
    const posBadge = document.querySelector('.positive-badge');
    nsfwToggle.addEventListener('change', () => {
        if (nsfwToggle.checked) {
            nsfwTabBtn.style.display = 'inline-block';
            posBadge.textContent = 'NSFW Mode';
            posBadge.className = 'badge positive-badge nsfw-mode';
        } else {
            nsfwTabBtn.style.display = 'none';
            posBadge.textContent = 'SFW';
            posBadge.className = 'badge positive-badge';
            if (nsfwTabBtn.classList.contains('active')) {
                document.querySelector('.tab-btn[data-tab="sfw"]').click();
            }
        }
    });

    document.getElementById('add-manual-positive-btn').addEventListener('click', () => {
        handleManualAdd(document.getElementById('manual-positive-input'), false);
    });
    document.getElementById('manual-positive-input').addEventListener('keypress', (event) => {
        if (event.key === 'Enter') handleManualAdd(event.currentTarget, false);
    });
    document.getElementById('add-manual-negative-btn').addEventListener('click', () => {
        handleManualAdd(document.getElementById('manual-negative-input'), true);
    });
    document.getElementById('manual-negative-input').addEventListener('keypress', (event) => {
        if (event.key === 'Enter') handleManualAdd(event.currentTarget, true);
    });

    document.getElementById('save-favorite-btn').addEventListener('click', handleSaveFavorite);
    document.getElementById('favorite-name-input').addEventListener('keypress', (event) => {
        if (event.key === 'Enter') handleSaveFavorite();
    });
    document.getElementById('export-favorites-btn').addEventListener('click', exportFavorites);
    document.getElementById('import-favorites-input').addEventListener('change', (event) => {
        importFavorites(event.currentTarget.files[0]);
        event.currentTarget.value = '';
    });

    document.getElementById('clear-positive-btn').addEventListener('click', () => {
        state.positiveTags.clear();
        state.tagElements.forEach((elements, key) => {
            if (key.startsWith('positive::')) elements.forEach(element => element.classList.remove('active'));
        });
        updateOutputs();
    });

    document.getElementById('clear-negative-btn').addEventListener('click', () => {
        state.negativeTags.clear();
        state.tagElements.forEach((elements, key) => {
            if (key.startsWith('negative::')) elements.forEach(element => element.classList.remove('active'));
        });
        updateOutputs();
    });

    document.getElementById('copy-positive-btn').addEventListener('click', () => {
        copyText(document.getElementById('copy-positive-btn'), document.getElementById('positive-prompt-output').value);
    });
    document.getElementById('copy-negative-btn').addEventListener('click', () => {
        copyText(document.getElementById('copy-negative-btn'), document.getElementById('negative-prompt-output').value);
    });

    setupTheme();
});
