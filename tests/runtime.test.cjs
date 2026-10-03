const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function fixture() {
    const handlers = new Map();
    const styles = [];
    class Element {
        constructor(timeline = false) { this.timeline = timeline; this.attributes = new Map(); }
        matches(selector) { return this.timeline && selector === '.timeline-slider[data-scrubber="true"]'; }
        setAttribute(name, value) { this.attributes.set(name, value); }
        removeAttribute(name) { this.attributes.delete(name); }
        addEventListener() {}
        appendChild() {}
        closest() { return null; }
    }
    const document = {
        head: { appendChild(node) { styles.push(node.textContent); } }, body: null,
        getElementById() { return null; }, querySelector() { return null; },
        createElement() { return new Element(); }, createElementNS() { return new Element(); },
        addEventListener(name, callback) { if (!handlers.has(name)) handlers.set(name, []); handlers.get(name).push(callback); }
    };
    vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../assets/runtime.js'), 'utf8'), {
        window: {}, document, Element, console, setTimeout, clearTimeout, Symbol
    });
    return { Element, styles, emit(name, event) { for (const handler of handlers.get(name) || []) handler(event); } };
}
const marker = 'data-croptix-pip1-pointer-focus';
test('mouse-selected timeline retains pointer focus when Space starts playback', () => {
    const f = fixture(); const timeline = new f.Element(true);
    f.emit('pointerdown', { button: 0, target: timeline });
    f.emit('keydown', { key: ' ', target: timeline });
    assert.ok(timeline.attributes.has(marker));
    f.emit('focusout', { target: timeline, relatedTarget: null });
    assert.ok(timeline.attributes.has(marker));
    f.emit('keydown', { key: 'Tab', target: timeline });
    assert.equal(timeline.attributes.has(marker), false);
    assert.ok(f.styles.some(css => css.includes(`[${marker}]:focus-visible`)));
});
test('moving focus to another element clears the mouse marker', () => {
    const f = fixture(); const timeline = new f.Element(true);
    f.emit('pointerdown', { button: 0, target: timeline });
    assert.ok(timeline.attributes.has(marker));
    f.emit('focusout', { target: timeline, relatedTarget: new f.Element() });
    assert.equal(timeline.attributes.has(marker), false);
});
