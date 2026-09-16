import { mkdir } from 'node:fs/promises';
import sharp from 'sharp';

await mkdir('static/images', { recursive: true });
await sharp('hero-source.png')
  .resize({ width: 1440 })
  .webp({ quality: 82 })
  .toFile('static/images/hero.webp');

// The social card is code-native typography over the project hero asset.
const overlay = Buffer.from(
  `<svg width="1200" height="630" xmlns="http://www.w3.org/2000/svg"><rect width="1200" height="630" fill="#07090d" opacity=".45"/><g font-family="Arial, sans-serif"><text x="80" y="130" fill="#a799ff" font-size="44" font-weight="bold">vut.</text><text x="80" y="295" fill="#f5f7fb" font-size="68" font-weight="bold">Build a brighter</text><text x="80" y="380" fill="#9b8cff" font-size="68" font-weight="bold">tomorrow with Vut.</text><text x="84" y="480" fill="#bac3d5" font-size="25">Simple. Powerful. Yours.</text></g></svg>`
);
await sharp('hero-source.png')
  .resize(1200, 630, { fit: 'cover' })
  .composite([{ input: overlay }])
  .png()
  .toFile('static/og.png');
console.log('Prepared hero.webp and og.png');
