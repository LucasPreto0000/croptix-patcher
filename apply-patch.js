#!/usr/bin/env node
'use strict';
// Independent CrOptix patcher. All browser fixes are defined below.
// No runtime dependency on any external PiP script.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');
const readline = require('node:readline/promises');

function browserFixes() {
    const installed = Symbol.for('croptix.integrated-fixes.installed')
    if (window[installed]) return
    window[installed] = true

    let pipSession = null
    let openingPip = false
    let hideControlsTimer = null
    let controlsObserver = null

    const installStyles = () => {
        if (document.getElementById('croptix-pip1-style')) return
        const style = document.createElement('style')
        style.id = 'croptix-pip1-style'
        style.textContent = `
            html[data-croptix-pip1-seeking] [data-testid="player-controls-root"] [data-testid="top-controls-autohide"],
            html[data-croptix-pip1-seeking] [data-testid="player-controls-root"] [data-testid="bottom-controls-autohide"] {
                opacity: 0 !important;
                pointer-events: none !important;
                transition-duration: 0ms !important;
            }
            #croptix-pip1-next svg,
            [data-testid="next-episode-button"] [data-testid="next-episode-icon"] {
                width: 20px !important;
                height: 20px !important;
                fill: currentColor;
            }
            [data-testid="bottom-left-controls-stack"] > div:has(> [data-testid="next-episode-button"]),
            #croptix-pip1-next { order: 1; margin-left: -7px; }
            [data-testid="bottom-left-controls-stack"] > [data-testid="jump-backward-button"],
            [data-testid="bottom-left-controls-stack"] > [data-testid="jump-forward-button"] { order: 2; }
            .timeline-slider[data-scrubber="true"][data-croptix-pip1-pointer-focus]:focus-visible {
                outline: none !important;
            }
            .timeline-slider[data-scrubber="true"][data-croptix-pip1-pointer-focus]:focus-visible::-webkit-slider-thumb {
                box-shadow: 0 0 0 2px rgba(0,0,0,.22), 0 2px 7px rgba(0,0,0,.38) !important;
            }
            .timeline-slider[data-scrubber="true"][data-croptix-pip1-pointer-focus]:focus-visible::-moz-range-thumb {
                box-shadow: 0 0 0 2px rgba(0,0,0,.22), 0 2px 7px rgba(0,0,0,.38) !important;
            }
        `
        document.head.appendChild(style)
    }
    if (document.head) installStyles()
    else document.addEventListener('DOMContentLoaded', installStyles, { once: true })

    let pointerFocusedTimeline = null
    const clearPointerFocus = () => {
        pointerFocusedTimeline?.removeAttribute('data-croptix-pip1-pointer-focus')
        pointerFocusedTimeline = null
    }
    document.addEventListener('pointerdown', (event) => {
        if (event.button !== 0) return
        clearPointerFocus()
        if (!(event.target instanceof Element) || !event.target.matches('.timeline-slider[data-scrubber="true"]')) return
        pointerFocusedTimeline = event.target
        pointerFocusedTimeline.setAttribute('data-croptix-pip1-pointer-focus', '')
    }, true)
    document.addEventListener('keydown', (event) => {
        if (event.key === 'Tab' && !event.altKey && !event.ctrlKey && !event.metaKey) clearPointerFocus()
    }, true)
    document.addEventListener('focusout', (event) => {
        if (event.target === pointerFocusedTimeline && event.relatedTarget instanceof Element) clearPointerFocus()
    }, true)

    const restoreSeekControls = () => {
        if (!hideControlsTimer && !controlsObserver) return
        document.documentElement.removeAttribute('data-croptix-pip1-seeking')
        clearTimeout(hideControlsTimer)
        hideControlsTimer = null
        controlsObserver?.disconnect()
        controlsObserver = null
    }
    const hideSeekControls = () => {
        const controls = document.querySelector('[data-testid="player-controls-root"] [data-testid="bottom-controls-autohide"]')
        if (!controls) return
        document.documentElement.setAttribute('data-croptix-pip1-seeking', '')
        clearTimeout(hideControlsTimer)
        hideControlsTimer = setTimeout(restoreSeekControls, 6000)
        controlsObserver?.disconnect()
        controlsObserver = new MutationObserver(() => {
            if (controls.getAttribute('data-overlay-visible') === 'false') restoreSeekControls()
        })
        controlsObserver.observe(controls, { attributes: true, attributeFilter: ['data-overlay-visible'] })
    }
    document.addEventListener('keydown', (event) => {
        if (event.altKey || event.ctrlKey || event.metaKey) return
        const target = event.target instanceof Element ? event.target : null
        const timeline = target?.matches('.timeline-slider[data-scrubber="true"]')
        if (!timeline && target?.closest('input, textarea, select, [contenteditable="true"]')) return
        const steps = { ArrowLeft: -5, ArrowRight: 5, j: -10, J: -10, l: 10, L: 10 }
        const step = steps[event.key]
        if (step === undefined || event.defaultPrevented) return
        const root = document.querySelector('[data-testid="player-controls-root"]')
        const video = pipSession?.window.document.querySelector('video') || document.querySelector('video')
        if (!root || !video || video.readyState === 0 || !Number.isFinite(video.duration)) return
        const position = Math.max(0, Math.min(video.duration, video.currentTime + step))
        try { video.currentTime = position } catch { return }
        event.preventDefault()
        event.stopImmediatePropagation()
        if (timeline) {
            target.valueAsNumber = position
            const percent = `${video.duration > 0 ? position / video.duration * 100 : 0}%`
            target.style.setProperty('--timeline-progress-percent', percent)
            target.style.setProperty('--moz-progress-gradient-percent', percent)
        }
        hideSeekControls()
    }, true)
    document.addEventListener('click', (event) => {
        const target = event.target instanceof Element ? event.target : event.target?.parentElement
        if (target?.closest('[data-testid="jump-forward-button"], [data-testid="jump-backward-button"]')) hideSeekControls()
    }, true)
    document.addEventListener('pointermove', restoreSeekControls, true)

    const getViewModels = () => {
        const root = document.querySelector('[data-testid="player-controls-root"]')
        if (!root) return null
        const key = Object.keys(root).find((name) => name.startsWith('__reactFiber$'))
        for (let fiber = key && root[key], depth = 0; fiber && depth < 80; fiber = fiber.return, depth++) {
            const viewModels = fiber.memoizedProps?.value?.viewModelContainer
            if (viewModels) return viewModels
        }
        return null
    }
    const nextButton = document.createElement('button')
    nextButton.id = 'croptix-pip1-next'
    nextButton.type = 'button'
    nextButton.title = 'Próximo episódio'
    nextButton.setAttribute('aria-label', 'Próximo episódio')
    const nextIcon = document.createElementNS('http://www.w3.org/2000/svg', 'svg')
    nextIcon.setAttribute('viewBox', '0 0 24 24')
    nextIcon.setAttribute('aria-hidden', 'true')
    const nextPath = document.createElementNS('http://www.w3.org/2000/svg', 'path')
    nextPath.setAttribute('d', 'M5 3L18 11.3571V3H20V21H18V12.6429L5 21V3Z')
    nextIcon.appendChild(nextPath)
    nextButton.appendChild(nextIcon)
    nextButton.addEventListener('click', (event) => {
        event.preventDefault()
        event.stopPropagation()
        const nextEpisodeVM = getViewModels()?.nextEpisodeVM
        if (nextEpisodeVM?._nextEpisodeGuid != null && typeof nextEpisodeVM.loadNextEpisode === 'function') {
            nextEpisodeVM.loadNextEpisode()
        } else {
            nextButton.title = 'Próximo episódio indisponível'
            setTimeout(() => { nextButton.title = 'Próximo episódio' }, 2500)
        }
    })
    const placeNextButton = () => {
        const stack = document.querySelector('[data-testid="bottom-left-controls-stack"]')
        const volume = stack?.querySelector('[data-testid="volume-slider-container"]')
        const play = stack?.querySelector('[data-testid="play-pause-button"]')
        if (!volume || !play) return
        nextButton.className = play.className
        if (stack.querySelector('[data-testid="next-episode-button"]')) {
            nextButton.remove()
            return
        }
        if (nextButton.parentElement !== stack || volume.nextElementSibling !== nextButton) volume.after(nextButton)
    }
    const watchPlayerControls = () => {
        placeNextButton()
        const stackSelector = '[data-testid="bottom-left-controls-stack"]'
        const observer = new MutationObserver((records) => {
            if (records.some((record) => {
                if (record.target instanceof Element && record.target.closest(stackSelector)) return true
                return [...record.addedNodes].some((node) => node instanceof Element && (node.matches(stackSelector) || node.querySelector(stackSelector)))
            })) placeNextButton()
        })
        observer.observe(document.body, { childList: true, subtree: true })
    }
    if (document.body) watchPlayerControls()
    else document.addEventListener('DOMContentLoaded', watchPlayerControls, { once: true })

    const markPosition = (node, name) => {
        if (!node?.parentNode) return null
        const parent = node.parentNode
        const marker = document.createComment(name)
        parent.insertBefore(marker, node)
        return { node, parent, marker }
    }
    const restorePosition = (position, fallbackParent) => {
        if (!position) return
        const { node, parent, marker } = position
        if (marker.parentNode?.isConnected) marker.parentNode.insertBefore(node, marker)
        else if (parent.isConnected) parent.appendChild(node)
        else if (fallbackParent?.isConnected) fallbackParent.appendChild(node)
        marker.remove()
    }
    const reportPipState = (active) => {
        document.querySelector('[data-testid="pip-button"]')?.setAttribute('aria-label', active ? 'Exit Picture-in-Picture' : 'Enter Picture-in-Picture')
    }
    const openDocumentPip = async (video) => {
        const aspectRatio = video.videoWidth && video.videoHeight ? video.videoWidth / video.videoHeight : 16 / 9
        const pipWindow = await window.documentPictureInPicture.requestWindow({ width: 640, height: Math.round(640 / aspectRatio) })
        const videoPosition = markPosition(video, 'croptix-pip1-video')
        if (!videoPosition) {
            pipWindow.close()
            throw new Error('Video has no parent to restore after PiP closes.')
        }
        const subtitles = document.querySelector('.libassjs-canvas-parent')
        const subtitlePosition = markPosition(subtitles, 'croptix-pip1-subtitles')
        const oldControls = video.controls
        let closed = false
        let resizeTimer = null
        const resize = () => window.dispatchEvent(new Event('resize'))
        const restore = () => {
            if (closed) return
            closed = true
            pipWindow.removeEventListener('resize', resize)
            pipWindow.removeEventListener('pagehide', restore)
            clearTimeout(resizeTimer)
            video.controls = oldControls
            restorePosition(videoPosition)
            restorePosition(subtitlePosition, videoPosition.parent)
            if (pipSession?.window === pipWindow) pipSession = null
            reportPipState(false)
            resize()
        }
        try {
            const style = pipWindow.document.createElement('style')
            style.textContent = `
                :root { color-scheme: dark; }
                * { box-sizing: border-box; }
                html, body { width: 100%; height: 100%; margin: 0; overflow: hidden; background: #000; }
                .croptix-pip1 { position: relative; width: 100%; height: 100%; overflow: hidden; background: #000; }
                .croptix-pip1 > video { position: absolute !important; inset: 0 !important; width: 100% !important; height: 100% !important; object-fit: cover !important; object-position: center !important; background: transparent !important; }
                .croptix-pip1 > .libassjs-canvas-parent { position: absolute !important; inset: 0 !important; width: 100% !important; height: 100% !important; z-index: 2; pointer-events: none !important; }
            `
            pipWindow.document.head.appendChild(style)
            const container = pipWindow.document.createElement('div')
            container.className = 'croptix-pip1'
            pipWindow.document.body.appendChild(container)
            container.appendChild(video)
            video.controls = true
            if (subtitles) container.appendChild(subtitles)
            pipWindow.addEventListener('resize', resize)
            pipWindow.addEventListener('pagehide', restore, { once: true })
            pipSession = { window: pipWindow, restore }
            reportPipState(true)
            resize()
            resizeTimer = setTimeout(resize, 100)
        } catch (error) {
            restore()
            pipWindow.close()
            throw error
        }
    }
    const togglePip = async () => {
        if (openingPip) return
        if (pipSession) {
            pipSession.window.close()
            pipSession.restore()
            return
        }
        if (window.documentPictureInPicture?.window) {
            window.documentPictureInPicture.window.close()
            return
        }
        if (document.pictureInPictureElement) {
            await document.exitPictureInPicture()
            return
        }
        const video = document.querySelector('video')
        if (!video) return
        video.removeAttribute('disablePictureInPicture')
        openingPip = true
        try {
            if (window.documentPictureInPicture?.requestWindow) {
                try {
                    await openDocumentPip(video)
                    return
                } catch (error) {
                    console.warn('[CrOptix PiP1] Document PiP failed; trying native PiP.', error)
                }
            }
            await video.requestPictureInPicture()
        } catch (error) {
            console.warn('[CrOptix PiP1] Picture-in-Picture failed.', error)
        } finally {
            openingPip = false
        }
    }
    document.addEventListener('click', (event) => {
        const target = event.target instanceof Element ? event.target : event.target?.parentElement
        if (!target?.closest('[data-testid="pip-button"]')) return
        event.preventDefault()
        event.stopPropagation()
        event.stopImmediatePropagation()
        void togglePip()
    }, true)
}

const FIXES = [
    'PiP com legendas, controles nativos, restauração e fallback',
    'Próximo episódio ao lado do volume; ícone e alinhamento',
    'Ocultar barra ao avançar; restaurar ao mover o mouse',
    'Avanço imediato em cada repetição de ←/→ e J/L'
];
const START = '// <CROPTIX-INDEPENDENT-FIXES v1>';
const END = '// </CROPTIX-INDEPENDENT-FIXES>';
const code = ';(' + browserFixes.toString() + ')();\n';
const hash = crypto.createHash('sha256').update(code).digest('hex');
const block = START + '\n// SHA256: ' + hash + '\n' + code + END + '\n';

async function chooseFolder() {
    if (process.platform === 'win32') {
        const picker = [
            '[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)',
            'Add-Type -AssemblyName System.Windows.Forms',
            '$dialog = New-Object System.Windows.Forms.FolderBrowserDialog',
            '$dialog.Description = "Selecione a pasta com katamari.js e manifest.json"',
            '$dialog.ShowNewFolderButton = $false',
            'try { if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { [Console]::Write($dialog.SelectedPath) } } finally { $dialog.Dispose() }'
        ].join('\n');
        const result = spawnSync('powershell.exe', ['-NoProfile', '-STA', '-EncodedCommand', Buffer.from(picker, 'utf16le').toString('base64')], { encoding: 'utf8', windowsHide: true });
        if (!result.error && result.status === 0) {
            const selected = result.stdout.trim();
            if (!selected) throw new Error('Seleção cancelada. Nenhum arquivo foi alterado.');
            return selected;
        }
        console.log('Não foi possível abrir o seletor. Informe a pasta no terminal.');
    }
    if (!process.stdin.isTTY) throw new Error('Informe a pasta como argumento: node apply-patch.js "C:\\pasta\\da\\extensao"');
    const terminal = readline.createInterface({ input: process.stdin, output: process.stdout });
    try { return (await terminal.question('Pasta da extensão: ')).trim().replace(/^"|"$/g, ''); }
    finally { terminal.close(); }
}

function prepare(folder) {
    const requested = path.resolve(folder);
    if (!fs.existsSync(requested)) throw new Error('A pasta não existe: ' + requested);
    const root = fs.realpathSync(requested);
    if (!fs.statSync(root).isDirectory()) throw new Error('O caminho informado não é uma pasta.');
    const files = ['katamari.js', 'manifest.json'].map(name => {
        const filename = path.join(root, name);
        if (!fs.existsSync(filename) || !fs.statSync(filename).isFile()) throw new Error('Arquivo esperado não encontrado: ' + name + '. Se houver dist, selecione essa pasta.');
        if (fs.realpathSync(filename) !== filename) throw new Error('Arquivo redirecionado por link: ' + name + '. Selecione a pasta real.');
        return { name, filename, before: fs.readFileSync(filename) };
    });
    const main = files[0].before.toString('utf8');
    let manifest;
    try { manifest = JSON.parse(files[1].before.toString('utf8').replace(/^\uFEFF/, '')); }
    catch { throw new Error('manifest.json inválido. Nenhum arquivo foi alterado.'); }
    if (!Array.isArray(manifest.content_scripts) || !manifest.content_scripts.some(s => s.world === 'MAIN' && Array.isArray(s.js) && s.js.includes('katamari.js'))) {
        throw new Error('Manifesto incompatível: katamari.js precisa ser um content script no mundo MAIN.');
    }
    new vm.Script(main, { filename: 'katamari.js (original)' });
    if (!main.includes('player-controls-root') || !main.includes('nextEpisodeVM')) {
        throw new Error('Katamari incompatível: os controles e nextEpisodeVM esperados não foram encontrados.');
    }
    let base = main;
    let already = false;
    if (main.startsWith(START + '\n')) {
        const finish = main.indexOf('\n' + END + '\n');
        if (finish < 0) throw new Error('Patch existente incompleto. Nenhum arquivo foi alterado.');
        const existing = main.slice(0, finish + END.length + 2);
        if (existing !== block) throw new Error('Patch existente diferente ou editado. Restaure o backup antes de substituir.');
        already = true;
        base = main.slice(existing.length);
    } else if (main.startsWith('// <CROPTIX-PATCHER-PIP1>')) {
        // Migrate the previous executable patch, without running that code.
        const match = /^\/\/ <CROPTIX-PATCHER-PIP1>\r?\n[\s\S]*?\/\/ <\/CROPTIX-PATCHER-PIP1>\r?\n/.exec(main);
        if (!match) throw new Error('Patch anterior incompleto. Nenhum arquivo foi alterado.');
        base = main.slice(match[0].length);
    } else if (main.includes(START) || main.includes('croptix.pip1.installed') || main.includes('croptix.integrated-fixes.installed')) {
        throw new Error('Correções incorporadas sem bloco reconhecido. Nenhum arquivo foi alterado.');
    }
    const removed = [];
    const separate = name => /(^|[\\/])pip1?\.js$/.test(name);
    for (const script of manifest.content_scripts) {
        if (!Array.isArray(script.js)) continue;
        for (const name of script.js) if (separate(name)) removed.push(name);
        script.js = script.js.filter(name => !separate(name));
    }
    for (const entry of manifest.web_accessible_resources || []) {
        if (!Array.isArray(entry.resources)) continue;
        for (const name of entry.resources) if (separate(name)) removed.push(name);
        entry.resources = entry.resources.filter(name => !separate(name));
    }
    const patched = already ? main : block + base;
    new vm.Script(patched, { filename: 'katamari.js (corrigido)' });
    files[0].after = Buffer.from(patched);
    files[1].after = removed.length ? Buffer.from(JSON.stringify(manifest, null, 2) + '\n') : files[1].before;
    return { root, files, already, removed };
}

function commit(plan) {
    const changes = plan.files.filter(f => !f.before.equals(f.after));
    if (!changes.length) return null;
    // Complete validation happened before creating backups or changing originals.
    const backupRoot = path.join(plan.root, '.croptix-node-backups');
    fs.mkdirSync(backupRoot, { recursive: true });
    const backup = fs.mkdtempSync(path.join(backupRoot, 'snapshot-'));
    for (const file of plan.files) fs.writeFileSync(path.join(backup, file.name), file.before, { flag: 'wx' });
    fs.writeFileSync(path.join(backup, 'info.json'), JSON.stringify({ folder: plan.root, created: new Date().toISOString(), fixes: FIXES }, null, 2));
    const staged = [];
    const written = [];
    try {
        for (const file of changes) {
            const temporary = file.filename + '.patch-' + crypto.randomBytes(8).toString('hex') + '.tmp';
            fs.writeFileSync(temporary, file.after, { flag: 'wx', mode: fs.statSync(file.filename).mode });
            staged.push({ file, temporary });
        }
        for (const item of staged) {
            if (!fs.readFileSync(item.file.filename).equals(item.file.before)) throw new Error('Arquivo alterado por outro programa: ' + item.file.name);
            fs.renameSync(item.temporary, item.file.filename);
            written.push(item.file);
        }
    } catch (error) {
        for (const file of written.reverse()) fs.writeFileSync(file.filename, file.before);
        throw new Error('Aplicação interrompida; arquivos modificados foram restaurados. ' + error.message);
    } finally {
        for (const item of staged) if (fs.existsSync(item.temporary)) fs.unlinkSync(item.temporary);
    }
    return backup;
}
function restoreBackup(folder) {
    const root = fs.realpathSync(path.resolve(folder));
    const backups = path.join(root, '.croptix-node-backups');
    if (!fs.existsSync(backups)) throw new Error('Nenhum backup deste patcher encontrado.');
    const candidates = fs.readdirSync(backups).filter(n => /^snapshot-[A-Za-z0-9]+$/.test(n)).map(name => {
        const dir = path.join(backups, name);
        const info = JSON.parse(fs.readFileSync(path.join(dir, 'info.json'), 'utf8'));
        return { dir, info };
    }).sort((a, b) => String(b.info.created).localeCompare(String(a.info.created)));
    if (!candidates.length) throw new Error('Nenhum snapshot disponível.');
    const chosen = candidates[0];
    if (chosen.info.folder !== root) throw new Error('O backup pertence a outra pasta.');
    const files = ['katamari.js', 'manifest.json'].map(name => ({ name, filename: path.join(root, name), before: fs.readFileSync(path.join(root, name)), after: fs.readFileSync(path.join(chosen.dir, name)) }));
    new vm.Script(files[0].after.toString('utf8'));
    JSON.parse(files[1].after.toString('utf8').replace(/^\uFEFF/, ''));
    // Refuse to overwrite a Katamari replaced by a later extension update.
    const current = files[0].before.toString('utf8');
    if (!current.startsWith(START + '\n')) throw new Error('O Katamari atual não contém este patch. Restauração automática cancelada.');
    let written = [];
    try {
        for (const file of files) { fs.writeFileSync(file.filename, file.after); written.push(file); }
    } catch (error) {
        for (const file of written.reverse()) fs.writeFileSync(file.filename, file.before);
        throw error;
    }
    console.log('katamari.js: restaurado.\nmanifest.json: restaurado.\nBackup utilizado: ' + chosen.dir);
    console.log('Recarregue a extensão e a página.');
}
async function main() {
    const args = process.argv.slice(2);
    if (args[0] === '--help') {
        console.log('Uso: node apply-patch.js "C:\\pasta\\da\\extensao"\nSem argumento: seletor de pasta no Windows, ou pergunta no terminal.');
        return;
    }
    const restore = args[0] === '--restore';
    if (restore) args.shift();
    if (args.length > 1) throw new Error('Informe apenas uma pasta como argumento.');
    const folder = args[0] || await chooseFolder();
    if (!folder) throw new Error('Pasta não informada.');
    if (restore) { restoreBackup(folder); return; }
    const plan = prepare(folder);
    const backup = commit(plan);
    console.log('Pasta: ' + plan.root);
    console.log('katamari.js:');
    for (const fix of FIXES) console.log('  ' + (plan.already ? 'Já aplicado: ' : 'Aplicado: ') + fix);
    console.log('manifest.json: ' + (plan.removed.length ? 'removido carregamento separado de ' + [...new Set(plan.removed)].join(', ') : 'já compatível; sem alteração.'));
    if (backup) console.log('Backup dos arquivos anteriores: ' + backup);
    else console.log('Nenhum arquivo alterado; patch já aplicado.');
    console.log('Recarregue a extensão e dê F5 na Crunchyroll.');
}
main().catch(error => { console.error('Erro: ' + error.message); process.exitCode = 1; });
