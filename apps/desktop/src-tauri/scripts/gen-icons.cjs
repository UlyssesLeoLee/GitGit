// Generate icon.ico (Windows) from icon.png using the modern PNG-in-ICO format.
const fs = require('fs');
const png = fs.readFileSync('icons/icon.png');
const ICONDIR = Buffer.alloc(6);
ICONDIR.writeUInt16LE(0, 0);
ICONDIR.writeUInt16LE(1, 2);
ICONDIR.writeUInt16LE(1, 4);
const ENTRY = Buffer.alloc(16);
ENTRY[0] = 0; ENTRY[1] = 0; ENTRY[2] = 0; ENTRY[3] = 0;
ENTRY.writeUInt16LE(1, 4);
ENTRY.writeUInt16LE(32, 6);
ENTRY.writeUInt32LE(png.length, 8);
ENTRY.writeUInt32LE(22, 12);
fs.writeFileSync('icons/icon.ico', Buffer.concat([ICONDIR, ENTRY, png]));
console.log('wrote icon.ico');

// Also create the smaller PNGs Tauri expects on some platforms (just symlinks to the existing icon for simplicity).
for (const size of [32, 128]) {
  fs.writeFileSync(`icons/${size}x${size}.png`, png);
  console.log(`wrote icons/${size}x${size}.png`);
}
fs.writeFileSync('icons/128x128@2x.png', png);
console.log('wrote icons/128x128@2x.png');