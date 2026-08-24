# Karto — Plan de medición de la landing (Umami)

Este documento define **qué medimos, por qué, y cómo se lee**. No es una lista de
métricas bonitas: cada dato aquí existe para responder una pregunta concreta de
producto o marketing. Si un número no cambia una decisión, no lo medimos.

> **Contexto que condiciona todo:** Karto está en v0.x, Linux-only, pre-lanzamiento.
> El tráfico llegará en **picos** (un post en r/selfhosted, un Show HN) seguidos de
> valles. Eso significa que **no perseguimos tendencias diarias suaves**: analizamos
> por *campaña/oleada*. Cada pico es un experimento con su propia audiencia.

> **Nota de coherencia de marca:** medimos la *web*, no la *app*. La promesa de
> "cero telemetría" es sobre el producto (`.karto` local, sin llamadas a casa) y se
> mantiene intacta. Un Umami self-hosted en la landing es, además, on-brand: analytics
> sin cookies, sin enviar datos a terceros. Si alguien pregunta, la respuesta es honesta.

---

## 1. La métrica que manda (North Star)

**Clicks en "Descargar para Linux".**

En pre-lanzamiento esto es la conversión real. No es el pageview, no es el tiempo en
página: es el gesto de intención de uso. Todo lo demás en este documento existe para
explicar por qué esta cifra sube o baja.

Objetivo operativo: **maximizar la tasa `clicks de descarga / visitantes únicos`**,
no el número absoluto (que depende del tamaño del pico de tráfico).

---

## 2. El embudo (así se lee la página de arriba a abajo)

Pensamos la landing como 4 escalones. En cada uno perdemos gente; el trabajo es saber
*dónde* se cae para arreglar *esa* sección, no la página entera.

| # | Escalón | Pregunta que responde | Señal en Umami |
|---|---------|----------------------|----------------|
| 1 | **Llega** | ¿De dónde viene y cuánta gente? | Visitas + referrer + UTM |
| 2 | **Entiende** | ¿El hero comunica qué es Karto? | % que hace scroll más allá del hero / rebote |
| 3 | **Se interesa** | ¿Qué parte del producto engancha? | Profundidad de scroll por acto |
| 4 | **Convierte** | ¿Da el paso? | Evento `download_click` |

La caída entre 2 y 3 es la más diagnóstica: si mucha gente entra y casi nadie pasa del
hero, el problema es el **mensaje**, no el producto ni el CTA.

---

## 3. Eventos a instrumentar

Umami captura pageviews, referrers y UTMs **automáticamente**. Lo que hay que añadir a
mano son los **eventos de intención**. En Umami se hace con el atributo
`data-umami-event="nombre"` en el elemento clicable (o `umami.track('nombre')` por JS).

### Eventos primarios (imprescindibles)

| Evento | Dónde | Por qué importa |
|--------|-------|-----------------|
| `download_hero` | Botón "Descargar para Linux" del hero | Conversión "en caliente": convenció solo con el hero |
| `download_final` | Botón "Descargar" del acto *Despega* | Conversión "convencida": leyó y decidió |
| `demo_scroll` | Enlace "Ver el mapa en acción" | Mide si el hero genera curiosidad aunque no descargue |

Separar `download_hero` de `download_final` es deliberado: la proporción entre ambos te
dice si el hero **vende solo** o si necesita que la página entera argumente. Los dos
suman la North Star.

### Eventos secundarios (contexto, no obsesión)

| Evento | Dónde | Qué revela |
|--------|-------|-----------|
| `faq_open` (con el texto de la pregunta) | Cada `<details>` de la FAQ | Las **objeciones reales**. Si "¿y si olvido la contraseña?" domina, el miedo es la pérdida de datos → dirígelo en el copy |
| `github_click` | Enlace a GitHub (footer) | Señal de público técnico/escéptico que quiere auditar. En homelab/self-hosted, esto correlaciona con adopción |
| `outbound_kofi` | Ko-fi | Intención de apoyo temprano |
| `scroll_navegacion` | Al entrar el acto "Navegación" en pantalla | Confirma que pasaron del mensaje emocional al funcional |
| `scroll_observatorio` | Al entrar el acto de seguridad | El público escéptico "profesional" que llega aquí es tu mejor lead |
| `scroll_descargar` | Al entrar la caja del CTA final en pantalla | Cierra el embudo de scroll: cuántos *llegan* al botón de abajo |

Los eventos de scroll usan un `IntersectionObserver` propio sobre los elementos con
`data-scroll` (independiente del de `data-reveal`, para que la analítica corra aunque
`prefers-reduced-motion` apague las animaciones). Cada uno dispara una sola vez.

El embudo de scroll se lee en cadena: `scroll_navegacion` → `scroll_observatorio` →
`scroll_descargar`. La caída entre el primero y el último te dice cuánta gente que
empezó a explorar el producto sobrevive hasta el CTA.

---

## 4. Fuentes de tráfico: etiqueta siempre con UTM

El tráfico de Karto vendrá de sitios identificables. **Nunca compartas un link desnudo.**
Cada canal lleva su UTM para poder comparar la *calidad* de cada comunidad, no solo el
volumen:

```
https://karto.app/?utm_source=reddit&utm_medium=social&utm_campaign=selfhosted-launch
https://karto.app/?utm_source=hn&utm_medium=social&utm_campaign=show-hn
https://karto.app/?utm_source=linkedin&utm_medium=social&utm_campaign=evesan
```

La pregunta que esto responde: *¿r/homelab manda más gente pero r/selfhosted convierte
mejor?* Volumen y conversión son cosas distintas; una comunidad pequeña que descarga
mucho vale más que una grande que rebota.

---

## 5. Lo que sí y lo que no

**Sí vigilar:**
- Tasa de conversión a descarga por fuente (UTM).
- Dónde muere el scroll (qué acto es el último que ve la mayoría).
- Qué pregunta de la FAQ se abre más (= la objeción #1 a resolver en el copy).
- Referrers orgánicos inesperados (alguien te enlazó → oportunidad).

**No obsesionarse (ruido en esta etapa):**
- Métricas diarias suaves: tu tráfico es a picos, la media diaria no dice nada.
- Tiempo en página como métrica de éxito: mucho tiempo puede ser interés *o* confusión.
  Solo vale cruzado con scroll y conversión.
- Micro-optimizar copy con A/B testing: sin tráfico sostenido, no hay significancia
  estadística. Esa fase llega **después** del release de mac/Windows. (Ver STRATEGY.md.)

---

## 6. Cómo se lee (ritual, no vigilancia)

No mires el dashboard a diario. El dato útil aparece **por oleada**:

1. **Antes de publicar** en una comunidad: anota fecha y UTM. Es el inicio del experimento.
2. **48–72h después** del pico: saca el reporte de *esa* campaña.
   - ¿Cuántos llegaron? ¿De dónde? ¿Cuántos descargaron (%)?
   - ¿Hasta qué acto llegó la mayoría?
   - ¿Qué FAQ se abrió más?
3. **Una conclusión por oleada**, escrita en una línea. Ejemplo:
   *"Show HN: 1.200 visitas, 4,1% descarga, scroll muere en Seguridad, FAQ top =
   open source → añadir badge AGPL más arriba."*

Ese registro acumulado es más valioso que cualquier gráfica: es el historial de qué
mensaje funcionó con qué audiencia.

---

## 7. Señales de referencia (qué es "bueno")

Órdenes de magnitud para una landing de herramienta técnica local-first, no leyes.
Sirven para saber si algo está roto, no para celebrar decimales:

- **Rebote en hero** > 70% con tráfico cualificado (de tu comunidad) → el hero no
  comunica; revisa H1/subhead antes que nada.
- **Conversión a descarga** 2–5% de visitantes es sano en pre-lanzamiento con público
  afín. Por debajo de 1% con tráfico de r/selfhosted es señal de fricción o desajuste
  de mensaje.
- **Llegada al acto de Seguridad** (`scroll_observatorio`): si el público técnico no
  llega aquí, estás perdiendo tu argumento más fuerte antes de mostrarlo.

---

## 8. Cómo está implementado

Todo vive en el layout compartido `src/layouts/Landing.astro`, así que cubre ES y EN a
la vez. Detalle de diseño: **no** usamos `data-umami-event` sino `umami.track()` manual,
porque el `<Button>` de `@karto/ui` no reenvía atributos `data-*` a su nodo. En su lugar,
cada elemento a medir lleva un atributo `data-track` (o `data-scroll`/`data-faq`) y un
listener delegado en el documento resuelve el evento. Ventaja: es independiente de la
versión de Umami y de los internos del componente.

- [x] `download_hero` — `<span data-track>` que envuelve el `<Button>` del hero.
- [x] `download_final` — `<span data-track>` que envuelve el `<Button>` de *Despega*.
- [x] `demo_scroll` — enlace "Ver el mapa en acción".
- [x] `faq_open` (+ dato `q`) — evento `toggle` de cada `<details>`, con clave estable
      independiente del idioma (`password` · `telemetry` · `platforms` · `license`).
- [x] `github_click` · `linkedin_click` · `outbound_kofi` · `lang_switch` — footer.
- [x] `scroll_navegacion` · `scroll_observatorio` · `scroll_descargar` —
      `IntersectionObserver` propio sobre `[data-scroll]`, una sola vez cada uno.
- [ ] Definir `download_hero` y `download_final` como **Goals** en el panel de Umami.
- [ ] Guardar la tabla de UTMs por canal antes del primer lanzamiento.

### Configuración (Umami self-hosted)

Los scripts se inyectan por variable de entorno (`apps/landing/.env`, ver `.env.example`).
Si faltan, no se carga nada y `track()` queda como no-op silencioso.

```
PUBLIC_UMAMI_SRC=https://umami.evesan.rocks/script.js
PUBLIC_UMAMI_RECORDER_SRC=https://umami.evesan.rocks/recorder.js   # opcional
PUBLIC_UMAMI_WEBSITE_ID=<uuid de la web en Umami>
```

> Si el build corre en CI, hay que darle estas mismas variables allí.

### Grabador de sesión (`recorder.js`)

Además de la analítica agregada, cargamos el grabador de sesión de Umami. Sirve para
*ver* dónde se atasca la gente (movimientos, clics, scroll), no solo contar dónde muere
el scroll — muy útil para diagnosticar tras una oleada de tráfico. Es opcional: se
desactiva dejando `PUBLIC_UMAMI_RECORDER_SRC` vacío.

**Nota de privacidad:** la grabación es más intrusiva que la analítica agregada.
Verifica que la instancia tenga el enmascarado de inputs activo (Umami lo trae por
defecto) para no capturar nada sensible y mantener la coherencia con el discurso de
privacidad del producto.
```