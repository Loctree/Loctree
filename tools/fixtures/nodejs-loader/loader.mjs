// Node.js ES Module loader hooks
// https://nodejs.org/api/esm.html#loaders
//
// These exports are NOT imported - they're invoked by Node.js runtime
// when you run: node --experimental-loader=./loader.mjs app.js
//
// Without runtime API detection, loctree would flag these as dead code.

/**
 * Resolve hook - maps import specifiers to file URLs
 * Invoked by Node.js for every import statement
 */
export async function resolve(specifier, context, nextResolve) {
    // Custom resolution logic
    if (specifier.startsWith('custom:')) {
        return {
            url: new URL(specifier.replace('custom:', 'file:///')).href,
            shortCircuit: true
        };
    }

    return nextResolve(specifier, context);
}

/**
 * Load hook - provides source code for resolved modules
 * Invoked by Node.js after resolve()
 */
export async function load(url, context, nextLoad) {
    // Custom module loading logic
    if (url.includes('/custom/')) {
        return {
            format: 'module',
            source: 'export default { custom: true };',
            shortCircuit: true
        };
    }

    return nextLoad(url, context);
}

/**
 * Global preload hook - runs before any modules are loaded
 * Invoked once at startup
 */
export function globalPreload(context) {
    return `
        console.log('Custom loader initialized');
    `;
}

/**
 * Initialize hook - called when loader is loaded
 * New in Node.js 18.6.0+
 */
export async function initialize(data) {
    console.log('Loader initialize hook called', data);
}
