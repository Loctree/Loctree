// Renderer module for HTML test fixture
export function render(element) {
    element.innerHTML = '<h1>Rendered!</h1>';
}

export function hydrateOnIdle(callback) {
    requestIdleCallback(callback);
}
