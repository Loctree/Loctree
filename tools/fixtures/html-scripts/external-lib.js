// External library file
const Vue = {
    createApp: function(config) {
        return {
            mount: function(selector) {
                console.log('Mounted to', selector);
            }
        };
    },
    hydrateOnIdle: function(callback) {
        requestIdleCallback(callback);
    }
};
