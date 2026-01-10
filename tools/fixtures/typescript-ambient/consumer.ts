// Test fixture: Consumer file that doesn't import the ambient exports
// This simulates real-world usage where ambient types are used via TypeScript
// but never explicitly imported

const app: Window['myApp'] = {
  version: '1.0.0'
};

console.log(app.version);
