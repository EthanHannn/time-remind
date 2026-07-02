import antfu from '@antfu/eslint-config'

export default antfu({
  vue: true,
  typescript: true,
  ignores: [
    'dist/**',
    'logs/**',
    'docs/**',
    'src-tauri/gen/**',
    'src-tauri/target/**',
    'src-tauri/**/*.toml',
    'src-tauri/Cargo.lock',
  ],
})
