import { readFileSync } from 'fs';
import { join } from 'path';
import { cwd } from 'process';
import typescript from '@rollup/plugin-typescript';

const pkg = JSON.parse(readFileSync(join(cwd(), 'package.json'), 'utf8'));

const packageExports = pkg.exports;

function buildConfig(input, output) {
   return {
      input,
      output: [
         {
            file: output.import,
            format: 'esm',
         },
         {
            file: output.require,
            format: 'cjs',
         },
      ],
      plugins: [
         typescript({
            declaration: true,
            declarationDir: `./${output.import.split('/')[1]}`,
         }),
      ],
      external: [
         /^@tauri-apps\/api/,
         ...Object.keys(pkg.dependencies || {}),
         ...Object.keys(pkg.peerDependencies || {}),
      ],
   };
}

export default [
   buildConfig('guest-js/index.ts', packageExports['.']),
   buildConfig('guest-js/mocks.ts', packageExports['./mocks']),
];
