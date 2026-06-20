// ==========================================================================
// State Management
// ==========================================================================

const state = {
    // Stores selected prompts
    positiveTags: new Set(), // Store prompt strings
    negativeTags: new Set(), // Store prompt strings
    
    // Track tag elements by prompt string to easily toggle active state programmatically
    tagElements: new Map(), // prompt -> DOM element
};

// ==========================================================================
// Helper Functions
// ==========================================================================

/**
 * Fetch prompts data from the Rust backend API
 */
async function fetchPromptsData() {
    try {
        const response = await fetch('/api/prompts');
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    } catch (e) {
        console.error('Failed to fetch prompts data from API', e);
        return { sfw: [], nsfw: [], negative: [] };
    }
}

/**
 * Fetch favorites data from the Rust backend API
 */
async function fetchFavoritesData() {
    try {
        const response = await fetch('/api/favorites');
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    } catch (e) {
        console.error('Failed to fetch favorites data from API', e);
        return [];
    }
}

/**
 * Render the favorites list
 */
function renderFavorites(presets) {
    const listEl = document.getElementById('favorites-list');
    if (!listEl) return;
    
    listEl.innerHTML = '';
    
    if (presets.length === 0) {
        listEl.innerHTML = '<div class="no-favorites">保存されたお気に入りはまだありません。</div>';
        return;
    }
    
    presets.forEach(preset => {
        const item = document.createElement('div');
        item.className = 'favorite-item';
        
        // Item click behavior: load preset
        item.addEventListener('click', () => {
            loadPreset(preset);
        });
        
        const content = document.createElement('div');
        content.className = 'favorite-item-content';
        
        const nameEl = document.createElement('div');
        nameEl.className = 'favorite-item-name';
        nameEl.textContent = '⭐ ' + preset.name;
        
        const metaEl = document.createElement('div');
        metaEl.className = 'favorite-item-meta';
        
        const posCount = preset.positive ? preset.positive.length : 0;
        const negCount = preset.negative ? preset.negative.length : 0;
        metaEl.innerHTML = `<span>ポジティブ: ${posCount}</span> <span>ネガティブ: ${negCount}</span>`;
        
        content.appendChild(nameEl);
        content.appendChild(metaEl);
        
        const actions = document.createElement('div');
        actions.className = 'favorite-item-actions';
        
        const deleteBtn = document.createElement('button');
        deleteBtn.className = 'fav-delete-btn';
        deleteBtn.innerHTML = '🗑️';
        deleteBtn.title = '削除';
        deleteBtn.addEventListener('click', async (e) => {
            e.stopPropagation(); // Prevent loading preset when deleting
            if (confirm(`お気に入り「${preset.name}」を削除してもよろしいですか？`)) {
                await handleDeleteFavorite(preset.name);
            }
        });
        
        actions.appendChild(deleteBtn);
        
        item.appendChild(content);
        item.appendChild(actions);
        listEl.appendChild(item);
    });
}

/**
 * Load a favorite preset into the current selection
 */
function loadPreset(preset) {
    state.positiveTags.clear();
    state.negativeTags.clear();
    
    // Clear all active classes
    state.tagElements.forEach(btn => btn.classList.remove('active'));
    
    // Load positive tags
    if (preset.positive && Array.isArray(preset.positive)) {
        preset.positive.forEach(tag => {
            state.positiveTags.add(tag);
            const btn = state.tagElements.get(tag);
            if (btn) {
                btn.classList.add('active');
            }
        });
    }
    
    // Load negative tags
    if (preset.negative && Array.isArray(preset.negative)) {
        preset.negative.forEach(tag => {
            state.negativeTags.add(tag);
            const btn = state.tagElements.get(tag);
            if (btn) {
                btn.classList.add('active');
            }
        });
    }
    
    updateOutputs();
}

/**
 * Save current tags as a favorite preset
 */
async function handleSaveFavorite() {
    const inputEl = document.getElementById('favorite-name-input');
    if (!inputEl) return;
    
    const name = inputEl.value.trim();
    if (!name) {
        alert('お気に入りの名前を入力してください。');
        return;
    }
    
    if (state.positiveTags.size === 0 && state.negativeTags.size === 0) {
        alert('プロンプトが選択されていません。タグを選択してから保存してください。');
        return;
    }
    
    const preset = {
        name: name,
        positive: Array.from(state.positiveTags),
        negative: Array.from(state.negativeTags)
    };
    
    try {
        const response = await fetch('/api/favorites', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(preset)
        });
        
        if (response.ok) {
            inputEl.value = '';
            // Refresh favorites list
            const updatedPresets = await fetchFavoritesData();
            renderFavorites(updatedPresets);
        } else {
            alert('保存に失敗しました。ファイル名に使用できない文字が含まれている可能性があります。');
        }
    } catch (e) {
        console.error('Failed to save favorite preset', e);
        alert('保存中にエラーが発生しました。');
    }
}

/**
 * Delete a favorite preset
 */
async function handleDeleteFavorite(name) {
    try {
        const response = await fetch(`/api/favorites/${encodeURIComponent(name)}`, {
            method: 'DELETE'
        });
        
        if (response.ok) {
            // Refresh favorites list
            const updatedPresets = await fetchFavoritesData();
            renderFavorites(updatedPresets);
        } else {
            alert('削除に失敗しました。');
        }
    } catch (e) {
        console.error('Failed to delete favorite preset', e);
        alert('削除中にエラーが発生しました。');
    }
}

// ==========================================================================
// Core UI Logic
// ==========================================================================

/**
 * Create a tag button in the DOM
 */
function createTagElement(item, isNegative) {
    const btn = document.createElement('button');
    btn.className = 'prompt-tag';
    
    const jpSpan = document.createElement('span');
    jpSpan.className = 'jp-name';
    jpSpan.textContent = item.japanese;
    
    const enSpan = document.createElement('span');
    enSpan.className = 'en-name';
    enSpan.textContent = item.prompt;
    
    btn.appendChild(jpSpan);
    btn.appendChild(enSpan);
    
    // Handle Click to toggle
    btn.addEventListener('click', (e) => {
        e.stopPropagation(); // Prevent accordion toggling if event bubbles
        toggleTag(item.prompt, isNegative);
    });
    
    // Save reference in state
    state.tagElements.set(item.prompt, btn);
    
    return btn;
}

/**
 * Add or remove a tag from selection
 */
function toggleTag(promptText, isNegative) {
    const targetSet = isNegative ? state.negativeTags : state.positiveTags;
    const btn = state.tagElements.get(promptText);
    
    if (targetSet.has(promptText)) {
        targetSet.delete(promptText);
        if (btn) btn.classList.remove('active');
    } else {
        targetSet.add(promptText);
        if (btn) btn.classList.add('active');
    }
    
    updateOutputs();
}

/**
 * Update outputs textareas based on state
 */
function updateOutputs() {
    const positiveTextarea = document.getElementById('positive-prompt-output');
    const negativeTextarea = document.getElementById('negative-prompt-output');
    
    positiveTextarea.value = Array.from(state.positiveTags).join(', ');
    negativeTextarea.value = Array.from(state.negativeTags).join(', ');
}

/**
 * Render category list as accordion cards
 */
function renderCategories(categories, containerEl, isNegative, defaultOpenFirst = false) {
    containerEl.innerHTML = '';
    
    if (categories.length === 0) {
        containerEl.innerHTML = '<div class="loading">データがありません。</div>';
        return;
    }
    
    categories.forEach((cat, index) => {
        const card = document.createElement('div');
        // Collapse all by default, except optionally the first one
        const isCollapsed = !(defaultOpenFirst && index === 0);
        card.className = `category-card${isCollapsed ? ' collapsed' : ''}`;
        
        // Header (Accordion Toggle)
        const title = document.createElement('div');
        title.className = 'category-title';
        title.textContent = cat.name;
        
        title.addEventListener('click', () => {
            card.classList.toggle('collapsed');
        });
        
        card.appendChild(title);
        
        // Body wrapper for grid height animation
        const body = document.createElement('div');
        body.className = 'category-body';
        
        // Inner body containing tags
        const inner = document.createElement('div');
        inner.className = 'category-body-inner';
        
        cat.tags.forEach(item => {
            const tagEl = createTagElement(item, isNegative);
            inner.appendChild(tagEl);
        });
        
        body.appendChild(inner);
        card.appendChild(body);
        containerEl.appendChild(card);
    });
}

// ==========================================================================
// Initialization & Event Listeners
// ==========================================================================

document.addEventListener('DOMContentLoaded', async () => {
    const sfwContainer = document.getElementById('sfw-categories');
    const nsfwContainer = document.getElementById('nsfw-categories');
    const negativeContainer = document.getElementById('negative-categories');
    
    // 1. Fetch all prompts data dynamically from API
    const data = await fetchPromptsData();
    
    // 2. Render categories
    renderCategories(data.sfw, sfwContainer, false, true); // SFW: open the first one by default
    renderCategories(data.nsfw, nsfwContainer, false, false); // NSFW: collapse all by default
    renderCategories(data.negative, negativeContainer, true, true); // Negative: open the first one by default
    
    // 2.5. Fetch and render favorites
    const favorites = await fetchFavoritesData();
    renderFavorites(favorites);
    
    // --- Tabs Navigation ---
    const tabBtns = document.querySelectorAll('.tab-btn');
    const tabPanels = document.querySelectorAll('.tab-panel');
    
    tabBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabName = btn.getAttribute('data-tab');
            
            tabBtns.forEach(b => b.classList.remove('active'));
            tabPanels.forEach(p => p.classList.remove('active'));
            
            btn.classList.add('active');
            document.getElementById(`panel-${tabName}`).classList.add('active');
        });
    });
    
    // --- NSFW Mode Toggle ---
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
            
            // If the active tab was NSFW, switch back SFW tab
            if (nsfwTabBtn.classList.contains('active')) {
                document.querySelector('.tab-btn[data-tab="sfw"]').click();
            }
        }
    });
    
    // --- Manual Tag Additions ---
    const addManualPositiveBtn = document.getElementById('add-manual-positive-btn');
    const manualPositiveInput = document.getElementById('manual-positive-input');
    const addManualNegativeBtn = document.getElementById('add-manual-negative-btn');
    const manualNegativeInput = document.getElementById('manual-negative-input');
    
    const handleManualAdd = (inputEl, isNegative) => {
        const text = inputEl.value.trim();
        if (!text) return;
        
        // Split by comma if user entered multiple tags
        const newTags = text.split(',').map(t => t.trim()).filter(t => t !== '');
        
        newTags.forEach(tag => {
            const targetSet = isNegative ? state.negativeTags : state.positiveTags;
            targetSet.add(tag);
        });
        
        updateOutputs();
        inputEl.value = '';
    };
    
    addManualPositiveBtn.addEventListener('click', () => handleManualAdd(manualPositiveInput, false));
    manualPositiveInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') handleManualAdd(manualPositiveInput, false);
    });
    
    addManualNegativeBtn.addEventListener('click', () => handleManualAdd(manualNegativeInput, true));
    manualNegativeInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') handleManualAdd(manualNegativeInput, true);
    });
    
    // --- Favorites Listeners ---
    const saveFavoriteBtn = document.getElementById('save-favorite-btn');
    const favoriteNameInput = document.getElementById('favorite-name-input');
    
    if (saveFavoriteBtn && favoriteNameInput) {
        saveFavoriteBtn.addEventListener('click', handleSaveFavorite);
        favoriteNameInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') handleSaveFavorite();
        });
    }
    
    // --- Clear Action Buttons ---
    document.getElementById('clear-positive-btn').addEventListener('click', () => {
        state.positiveTags.clear();
        state.tagElements.forEach((btn, prompt) => {
            const isNegativePanelTag = btn.closest('#panel-negative');
            if (!isNegativePanelTag) {
                btn.classList.remove('active');
            }
        });
        updateOutputs();
    });
    
    document.getElementById('clear-negative-btn').addEventListener('click', () => {
        state.negativeTags.clear();
        state.tagElements.forEach((btn, prompt) => {
            const isNegativePanelTag = btn.closest('#panel-negative');
            if (isNegativePanelTag) {
                btn.classList.remove('active');
            }
        });
        updateOutputs();
    });
    
    // --- Copy Clipboard buttons with visual indication ---
    const handleCopy = (btnEl, textValue) => {
        if (!textValue) return;
        
        navigator.clipboard.writeText(textValue).then(() => {
            btnEl.classList.add('copied');
            const copyText = btnEl.querySelector('.copy-text');
            const successText = btnEl.querySelector('.copy-success-text');
            
            copyText.style.display = 'none';
            successText.style.display = 'inline';
            
            setTimeout(() => {
                btnEl.classList.remove('copied');
                copyText.style.display = 'inline';
                successText.style.display = 'none';
            }, 2000);
        }).catch(err => {
            console.error('Could not copy text: ', err);
        });
    };
    
    document.getElementById('copy-positive-btn').addEventListener('click', () => {
        const text = document.getElementById('positive-prompt-output').value;
        handleCopy(document.getElementById('copy-positive-btn'), text);
    });
    
    document.getElementById('copy-negative-btn').addEventListener('click', () => {
        const text = document.getElementById('negative-prompt-output').value;
        handleCopy(document.getElementById('copy-negative-btn'), text);
    });

    // --- Theme Toggle Logic ---
    const themeToggleBtn = document.getElementById('theme-toggle-btn');
    const sunIcon = themeToggleBtn.querySelector('.theme-icon-sun');
    const moonIcon = themeToggleBtn.querySelector('.theme-icon-moon');
    
    // Check saved theme or default to dark
    const savedTheme = localStorage.getItem('theme') || 'dark';
    document.documentElement.setAttribute('data-theme', savedTheme);
    updateThemeUI(savedTheme);
    
    themeToggleBtn.addEventListener('click', () => {
        const currentTheme = document.documentElement.getAttribute('data-theme');
        const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
        
        document.documentElement.setAttribute('data-theme', newTheme);
        localStorage.setItem('theme', newTheme);
        updateThemeUI(newTheme);
    });
    
    function updateThemeUI(theme) {
        if (theme === 'light') {
            sunIcon.style.display = 'none';
            moonIcon.style.display = 'inline';
        } else {
            sunIcon.style.display = 'inline';
            moonIcon.style.display = 'none';
        }
    }

    const openFavoritesBtn = document.getElementById('open-favorites-btn');
    if (openFavoritesBtn) {
        openFavoritesBtn.addEventListener('click', async () => {
            try {
                await fetch('/api/open_favorites', { method: 'POST' });
            } catch (e) {
                console.error('Failed to open favorites folder:', e);
            }
        });
    }
    // --- Heartbeat to keep backend alive ---
    setInterval(() => {
        fetch('/api/ping').catch(e => console.error('Ping failed:', e));
    }, 2000); // Ping every 2 seconds
});
