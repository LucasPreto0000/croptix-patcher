;(function browserFixes() {    const installed = Symbol.for('croptix.integrated-fixes.installed')
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
        `
        document.head.appendChild(style)
    }
    if (document.head) installStyles()
    else document.addEventListener('DOMContentLoaded', installStyles, { once: true })

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

})();
