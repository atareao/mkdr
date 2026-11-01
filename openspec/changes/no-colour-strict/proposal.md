# Proposal

## Why

`--no-colour` promete «Disable all colours and other styles», pero en v0.6.4 solo sustituye el tema por `Theme::plain()` (un tema sin claves). El `Renderer` reinyecta estilos por fallback cuando el tema no define una clave —negrita `true`, cursiva `true`, tachado `true`, código inline `Color::Yellow`, enlaces `Color::Cyan` + subrayado, bordes de tabla `Color::DarkGray`, títulos `bold: i < 3`— y varias capas de la TUI (barra de estado, lista de ficheros, numeración, vista raw, resaltado de búsqueda) usan colores y modificadores literales. El resultado es que el flag no cumple lo que documenta: en un pty real, con `--no-colour` siguen llegando bold, italic, underline, dim y varios colores reales al terminal. Los tests actuales no lo detectan porque solo comprueban `fg`/`bg`, nunca los modificadores, y su fixture no incluye código inline, enlaces ni tablas.

## What Changes

- La supresión de estilos pasa a ser una política de renderizado explícita, solicitada con `--no-colour` / `-c` y propagada desde la CLI, en lugar de inferirse de si el tema tiene estilos.
- Los fallbacks de estilo del renderer dejan de aplicarse cuando la política está activa.
- Se suprimen los colores literales de las construcciones del documento: código inline y bloques de código (incluido el resaltado de sintaxis), enlaces, bordes de tabla, viñetas, marcas de lista numerada, marcas de cita, reglas horizontales y checkboxes de task list.
- El chrome de la TUI (barra de estado, lista de ficheros, numeración de líneas y estilo de frontmatter en la vista raw) respeta la política.
- El resaltado de coincidencias de búsqueda no aplica estilo cuando la política está activa, sin perder la funcionalidad de búsqueda.
- Sin el flag, el comportamiento observable no cambia.
- Se cierra el hueco de test: las aserciones cubren modificadores además de `fg`/`bg`, y se añade una comprobación a nivel de buffer completo del frame.

## Capabilities

### New Capabilities
- `terminal-styling`: política de estilo de la salida de la TUI — cuándo se aplican colores, énfasis y resaltados, y qué se garantiza cuando la supresión de estilos está activa.

### Modified Capabilities

Ninguna. `openspec/specs/` está vacío: este es el primer change del repositorio y no hay requisitos previos que modificar.

## Impact

- Código: `src/render.rs` (construcción del renderer y puntos de uso de color), `src/app.rs` (propagación del flag, barra de estado, lista de ficheros, vista raw, numeración), `src/search.rs` (resaltado de búsqueda), `src/main.rs` (paso del flag a la aplicación).
- API interna: la construcción de la aplicación y la función de resaltado de búsqueda ganan un parámetro de política; ninguna función pública de librería cambia.
- CLI: sin cambios en flags, ayuda ni formato de salida; solo cambia el efecto real de `--no-colour`.
- Tests: nuevos tests unitarios por módulo y un test de frame completo con backend de pruebas; se amplía el test existente de supresión de estilos.
- Dependencias: ninguna nueva. Temas: sin cambios en el esquema ni en los temas incluidos.
