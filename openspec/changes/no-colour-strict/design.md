# Design

## Context

Estado actual, verificado sobre el binario publicado v0.6.4 ejecutado en un pty:

- `src/main.rs` sustituye el tema por `Theme::plain()` cuando llega `--no-colour`.
- `src/app.rs` deduce el modo sin estilos de `theme.is_plain()` (tema sin claves), no del flag. Ese booleano se guarda como campo y decide qué renderizador se usa.
- `Renderer::new` (`src/render.rs`) resuelve cada estilo del tema y, cuando la clave no existe, aplica un fallback literal: negrita, cursiva y tachado con su modificador activo; código inline en amarillo; enlaces en cian y subrayados; bordes de tabla en gris oscuro; títulos en negrita.
- Varios puntos de uso de `render.rs` aplican colores literales: viñetas, marcas de listas numeradas, marca de cita (verde verdadero), reglas horizontales y checkboxes de task list.
- En `app.rs`, la barra de estado, la lista de ficheros, la numeración de líneas y el frontmatter de la vista raw usan colores y negrita literales. `src/search.rs` resalta las coincidencias con fondo azul, texto blanco y negrita.
- El test existente de supresión de estilos solo comprueba color de primer plano y de fondo de los spans, con un documento que no incluye código inline, enlaces ni tablas.

Consecuencia medida: con `--no-colour`, el documento sigue emitiendo negrita, cursiva, subrayado, atenuado y colores como amarillo (código inline), cian (enlaces), gris oscuro (viñetas, listas, bordes de tabla) y verde verdadero (marca de cita); la barra de estado emite negrita, fondo azul y texto claro.

## Goals / Non-Goals

**Goals:**

- Que `--no-colour` garantice ausencia total de atributos de estilo en toda la salida de la TUI: documento, chrome y resaltado de búsqueda.
- Que la política de estilo se derive del flag solicitado por el usuario, no de una propiedad del tema.
- Que el comportamiento sin el flag permanezca intacto.
- Que la propiedad quede cubierta por tests que la verifiquen sobre el conjunto del frame, no solo sobre el documento.

**Non-Goals:**

- Soporte de la variable de entorno `NO_COLOR`: es una mejora deseable, pero es un change aparte.
- Cambiar el esquema de temas, los temas incluidos o la resolución de temas.
- Cambiar la CLI: no se añaden, quitan ni renombran flags; tampoco cambia el texto de la ayuda.
- Optimizar el rendimiento del renderizado o del resaltado de sintaxis.
- Cambiar el comportamiento de `--images`, `--columns`, `--line` o `--wrap`.

## Decisions

### 1. Política explícita en lugar de tema vacío

La causa raíz es que la ausencia de estilos se expresó como «tema sin claves», y a esa señal se le superponen fallbacks y colores literales que no consultan el tema. Se sustituye por una política explícita: la aplicación recibe un booleano de supresión de estilos procedente del flag y lo conserva como estado propio.

Alternativa descartada: mantener `Theme::plain()` y tratar «tema vacío» como «sin estilos» dentro del renderer. Se descarta porque confunde dos cosas distintas: un tema que no define claves y un usuario que pide explícitamente salida sin estilos. Además, con ese enfoque un tema de usuario incompleto cambiaría de comportamiento sin que el usuario lo pidiera.

### 2. Un único punto de decisión por capa

Se concentra la decisión en dos ayudantes pequeños, uno en el renderer del documento y otro en la aplicación, que devuelven el estilo por defecto cuando la política está activa y el estilo solicitado cuando no lo está. Los fallbacks de `Renderer::new` pasan a ser también condicionales: con la política activa, la ausencia de clave en el tema resuelve a estilo por defecto en lugar de a un fallback literal. En el chrome se aplica el mismo ayudante a la barra de estado, la lista de ficheros, la numeración y el frontmatter.

Alternativa descartada: recorrer cada punto de uso y añadir un `if` ad hoc. Se descarta porque multiplica las oportunidades de olvidar un sitio y hace imposible razonar sobre la propiedad global.

### 3. Resaltado de búsqueda monocromo

Con la política activa, las coincidencias de búsqueda no llevan estilo alguno. La búsqueda sigue siendo utilizable porque el cursor salta a la coincidencia y la barra de estado muestra el término y la posición alcanzada.

Alternativa descartada: conservar la negrita o el subrayado como pista estructural en las coincidencias. Se descarta porque el flag promete eliminar todos los estilos y la negrita lo es; mantenerla haría que la garantía del requisito dejara de ser comprobable como ausencia total de atributos.

### 4. Verificación sobre el frame completo

La comprobación central no es por construcción del documento, sino sobre el buffer dibujado de un frame completo con un backend de pruebas en memoria: se exige que todas las celdas tengan el estilo por defecto. Es una verificación fiel del objetivo físico, porque el backend de terminal solo emite secuencias SGR cuando una celda tiene un estilo distinto del por defecto. Se conservan además las aserciones por módulo, que localizan el fallo cuando la comprobación global se rompe.

La verificación de aceptación final se hace con una ejecución real en un pty sobre un documento con todas las construcciones, contando en la **salida cruda del pty** las secuencias SGR **que aplican color o modificador**: cero con el flag frente a del orden de varias decenas sin él (≈73 en la medición de referencia, según documento y geometría del terminal).

El criterio no puede ser «cero secuencias SGR» en sentido literal: el teardown de ratatui/crossterm emite siempre cuatro secuencias neutras de reinicio —`ESC[39m ESC[49m ESC[59m ESC[0m`— tanto con el flag como sin él, y no aplican ni color ni modificador. En una captura hecha a través de tmux el recuento de esos reinicios da 0 porque tmux normaliza los reseteos, de ahí que el criterio se refiera explícitamente al flujo crudo del pty.

### 5. Se conserva la sustitución por tema vacío

`main.rs` sigue pasando el tema vacío cuando llega el flag, además de la nueva política. No aporta nada a la garantía —los ayudantes la imponen— pero tampoco estorba y mantiene mínima la superficie del cambio. La única fuente de verdad para la garantía pasa a ser, en cualquier caso, el flag.

## Risks / Trade-offs

- **Pérdida de visibilidad de las coincidencias de búsqueda**: sin resaltado, localizar la coincidencia en una línea larga es más difícil. Mitigación: la barra de estado informa del término y de la posición, y el cursor salta a la línea. Contrapartida aceptada por coherencia con lo que promete el flag.
- **Superficie de estilo fácilmente ampliable**: cualquier construcción nueva que se añada en el futuro puede volver a introducir literales. Mitigación: la verificación sobre el frame completo falla automáticamente si eso ocurre, y una tarea de revisión busca literales de color y de modificador fuera de los ayudantes.
- **Firma de construcción de la aplicación**: añadir un parámetro a una función que ya acumula muchos argumentos. Mitigación: la función ya tiene declarada la excepción de linter correspondiente; agrupar las opciones en una estructura queda fuera del alcance de este change.
- **Falso positivo en la comprobación global**: un frame con un tamaño fijo y un estado concreto (por ejemplo sin búsqueda) no ejercita todos los puntos de uso. Mitigación: los escenarios dibujan más de un frame, incluyendo el modo búsqueda y el modo lista de ficheros.
- **Cambio de comportamiento no intencionado sin el flag**: el riesgo real es tocar los fallbacks y las rutas por defecto. Mitigación: los tests de regresión comprueban que sin la política los estilos siguen aplicándose, y la comparación del recuento de secuencias SGR de la versión por defecto antes y después de la ejecución del change.
