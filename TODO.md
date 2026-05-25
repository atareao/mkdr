# TODO

## Renderizado

- [x] **Syntax highlighting en bloques de código** — usar `syntect` con temas, coloreando según el lenguaje
- [x] **Blockquotes multi-línea** — barra vertical `▐` continua en cada línea envuelta
- [x] **Links con URL** — mostrar la URL en gris tenue tras el texto (`[texto](url)` → `texto ─ url`)
- [x] **Task lists** — `- [x]` / `- [ ]` con íconos ☑/☐ y estilo
- [x] **Imágenes** — mostrar `![alt](url)` como link con ícono
- [x] **Tablas: alineación** — respetar alineación izquierda/derecha/centro
- [x] **Enlaces por referencia** — `[text][ref]` + `[ref]: url` (pulldown-cmark resuelve internamente)

## Navegación

- [x] **Historial de búsqueda** — `↑`/`↓` recupera búsquedas anteriores (50 últimas)
- [x] **Marcadores** — `m{a-z}` marca línea, `'{a-z}` salta a marca
- [x] **Ir a porcentaje** — `:50%` salta al 50% del documento

## UX

- [x] **Auto-crear `~/.config/mkdr/themes/`** si no existe al arrancar
- [x] **Completado de shell** — `mkdr --completions bash`
- [x] **Pipe status bar** — mostrar "stdin" en barra al leer de pipe
- [x] **Múltiples archivos: lista** — `:files` muestra índice navegable con `j`/`k`/`Enter`
- [x] **Temas desde TUI** — `:theme nord` cambia sin reiniciar
- [x] **Recargar configuración** — `:reload` recarga config + theme

## Testing

- [x] **Tests de integración** — en `tests/` para la app completa (CLI args, archivo no encontrado, stdin pipe)
- [x] **Tests de snapshot: más casos** — listas anidadas, blockquote anidado, ordered list, task list, image, tablas complejas, contenido mixto con spacing, inline_code bg
- [x] **Tests de search** — búsqueda + highlight sobre contenido renderizado

## Empaquetado

- [x] **Completado bash/zsh/fish** en CI — generado en release y subido como asset
- [x] **Homebrew formula** — en `packaging/Formula/mkdr.rb`
- [x] **AUR package** — `packaging/PKGBUILD` para `mkdr-bin`
- [x] **cargo-binstall** — compatible sin configuración adicional (binario único `mkdr`)

## Bugs conocidos

- [x] **Wrap + tablas** — con `--wrap word`, las tablas se rompen (se desactiva wrap automáticamente)
- [x] **Archivo no encontrado** — mensaje de error antes de entrar a raw mode, sugerencia de uso