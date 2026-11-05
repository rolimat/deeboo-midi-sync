# Deeboo MIDI Sync — Documento de diseño

Fecha: 2026-11-05 · Estado: aprobado (2026-11-05)

## 1. Intención y porqué

El usuario pincha con **Traktor** enviando MIDI clock a **Ableton Live**. El tempo se
sincroniza, pero las **frases y beats no quedan en fase**: el MIDI clock solo lleva
tempo (24 pulsos por negra, `0xF8`), no la posición del beat 1, así que el esclavo queda
desplazado.

Deeboo MIDI Sync es una **app de barra de menú para macOS** que se pone en medio:

```
Software maestro ──clock──▶ [Deeboo In]  Deeboo  [Deeboo Out] ──clock──▶ Software esclavo
```

Recibe el clock por un puerto MIDI virtual de entrada y lo reenvía por un puerto virtual
de salida, permitiendo **pausar/reanudar** el envío y **empujar o frenar** el clock de
salida "como un vinilo" para poner en fase ambos programas. Muestra BPM y el beat (1–4)
en el que está la salida, como un CDJ.

Debe ser **genérica**: nada en la app menciona Traktor ni Ableton; cualquier software o
dispositivo puede conectarse a In/Out.

## 2. Objetivos

1. Crear dos puertos MIDI **virtuales** (CoreMIDI) con nombres configurables
   (por defecto `Deeboo In` y `Deeboo Out`).
2. Reenviar el MIDI clock de In a Out con la mínima latencia y jitter posible.
3. **Play/Pausa** del reenvío.
4. **Empuje fino** (Push ▶▶ / Slow down ◀◀) mientras se mantiene pulsado.
5. Mostrar **BPM** medido, **beat 1–4** de la salida, y estado de In/Out.
6. Icono en la barra de menú con indicador de señal parpadeando al beat y BPM.
7. Distribución como **`.dmg`** sin firmar.
8. Desarrollo contenido en Docker con linters, tests, hooks y CI.

## 3. No-objetivos

- Ajuste grueso (saltar 1 beat / 1 compás de golpe). El usuario lo descartó explícitamente.
- Contador de compás (`12.3`) y aviso de desplazamiento acumulado en la UI.
- Generar clock propio sin maestro (Deeboo no es un reloj maestro).
- Reenviar otros mensajes MIDI (notas, CC, SysEx). Solo mensajes de tiempo real del reloj
  (ver §5.2).
- Windows / Linux como plataformas de uso (Linux solo como entorno de compilación/test).
- Firma de código y notarización de Apple.
- Selección de puertos físicos existentes: solo puertos virtuales propios.

## 4. Anti-objetivos (sería un fracaso aunque "funcione")

- Que el reenvío añada jitter o latencia perceptibles con respecto a conectar directamente.
- Que el empuje cambie el **tempo de forma permanente**: al soltar, la salida vuelve a ir
  1:1 con la entrada, conservando solo el desplazamiento de fase ganado.
- Que la UI nombre software concreto.
- Que la app aparezca en el Dock o pida atención (es una utilidad de fondo).

## 5. Comportamiento

### 5.1 Puertos

- Al arrancar, Deeboo crea una destination virtual **In** y una source virtual **Out**
  (vía `midir::os::unix::{VirtualInput, VirtualOutput}`).
- Renombrar un puerto en Settings cierra y recrea ese puerto con el nuevo nombre
  (el software conectado tendrá que reseleccionarlo; aceptable).
- Si falla la creación de un puerto: su indicador queda en rojo y el error se registra en log.

### 5.2 Reenvío y Play/Pausa (acordado con el usuario)

Estado `playing: bool`, inicial **pausado** *(decisión del diseñador)*.

- **Pausa** (pulsar ⏸ estando en Play): deja de reenviar clock y envía `Stop (0xFC)` por Out.
- **Play** (pulsar ▶ estando en Pausa): envía `Start (0xFA)` por Out, pone el contador de
  beats a 1 en ese instante y reanuda el reenvío de `0xF8`.
- **El usuario es el único que inicia la reproducción del esclavo** (acordado con el usuario),
  para no perder nunca la fase ajustada con el empuje:
  - `Stop (0xFC)` entrante estando en Play → se reenvía `Stop` por Out y Deeboo pasa a **Pausa**.
  - `Start (0xFA)`, `Continue (0xFB)` y `Song Position Pointer (0xF2)` entrantes → **no se
    reenvían** nunca. El esclavo solo arranca con el Play de Deeboo.
  - **Pérdida de clock** (> 500 ms sin `0xF8`) estando en Play → Deeboo envía `Stop` por Out y
    pasa a **Pausa**. Al volver la señal no se reanuda sola: el usuario pulsa Play.
- `playing` cambia solo por: botón Play/Pausa, `Stop` entrante o pérdida de clock.
- En Pausa no se reenvía nada, pero **se sigue midiendo** el BPM y el estado de In.
- Los mensajes de tiempo real pueden llegar intercalados dentro de un paquete CoreMIDI: el
  parser recorre **todos los bytes** de cada paquete, no solo el primero.

Uso típico: pausar, esperar a que en el maestro llegue el beat 1 de la frase, pulsar Play
justo ahí, y afinar con ◀◀ / ▶▶.

### 5.3 Empuje fino ("vinilo")

- Mientras se **mantiene pulsado** ▶▶ (Push), el clock de salida va un **k %** más rápido
  que el de entrada; mientras se mantiene ◀◀ (Slow down), un k % más lento. `k` se configura
  en Settings, por defecto **4 %**, rango sugerido 1–10 %.
- Al soltar, la salida vuelve a 1:1; el desplazamiento de fase acumulado se conserva.
- Solo actúa en Play. En Pausa los botones están deshabilitados.
- **Mecanismo** *(decisión del diseñador; el constructor puede revisarlo tras probar con un
  esclavo real)*: inserción/eliminación de pulsos.
  - Push: un acumulador suma `k` por cada pulso de entrada; al superar 1 se programa **un pulso
    extra** a mitad del intervalo entre pulsos (intervalo medido actual), en un hilo temporizador
    dedicado. Así no salen dos pulsos pegados.
  - Slow down: el mismo acumulador; al superar 1 se **descarta** el siguiente pulso de entrada.
  - El contador de beats cuenta **pulsos de salida**, por lo que refleja lo que ve el esclavo.

### 5.4 Medición de BPM

- BPM = 60 / (24 × intervalo medio entre `0xF8` entrantes), media móvil de los últimos
  24 intervalos (1 beat), mostrado con 1 decimal.
- Timestamps: los que entrega midir (CoreMIDI host time) si están disponibles; si no, reloj
  monotónico en el callback.
- Sin `0xF8` durante **500 ms** → se considera "sin señal": BPM se muestra como `—`.

### 5.5 Estado de los indicadores

| Indicador | Verde | Rojo |
|---|---|---|
| In | Llegan pulsos de clock (último hace < 500 ms) | Sin señal o puerto no creado |
| Out | En Play, puerto creado y reenviando | En Pausa, o puerto no creado |

## 6. Interfaz

Todos los textos de la app en **inglés**. Mockup aprobado: `.superpowers/brainstorm/…/content/layout-v3.html`
(estilo oscuro, tarjetas redondeadas, inspirado en la captura de referencia del usuario).

### 6.1 Barra de menú

`[icono vinilo] [● punto] 124.0`

- Icono: **vinilo** (opción A elegida: disco con surcos y marca de agarre), monocromo y
  legible en barra clara y oscura.
- Punto: **verde** si In recibe señal, **rojo** si no. Parpadea una vez por **beat de entrada**
  (cada 24 pulsos recibidos) *(decisión del diseñador: el parpadeo refleja la señal de In)*.
  Sin señal: rojo fijo.
- BPM con 1 decimal, ancho de dígitos fijo (no "baila"); `—` sin señal.
- Clic → abre/cierra el popover. Implementación sugerida: icono compuesto en tiempo de
  ejecución (vinilo + punto) en variantes encendido/atenuado × verde/rojo, y BPM con
  `TrayIcon::set_title`. Cómo mantener el vinilo legible en barra clara/oscura con un punto
  de color queda al constructor.

### 6.2 Popover (vista principal)

Ventana sin bordes anclada bajo el icono (`tauri-plugin-positioner`), se oculta al perder el
foco. Sin icono en el Dock (`ActivationPolicy::Accessory`).

```
◉ Deeboo MIDI Sync                         ⚙
┌──────────────────────────────────────────┐
│ 124.0                                    │
│ BPM                                      │
│ [■ 1 ][■ 2 ][■ 3 ][□ 4 ]   (1 en rojo)    │
│ [  ◀◀  ]   [   ▶ / ⏸   ]   [  ▶▶  ]       │
└──────────────────────────────────────────┘
┌──────────────────────────────────────────┐
│ ● In · Deeboo In                         │
│ ● Out · Deeboo Out                       │
└──────────────────────────────────────────┘
```

- Botones **solo con iconos**. ◀◀ y ▶▶ actúan mientras se mantienen pulsados
  (pointerdown → empieza, pointerup/pointerleave/blur → para). Play muestra ▶ en Pausa
  y ⏸ en Play.
- Indicador de beats 1–4: se iluminan los beats hasta el actual del compás; el 1 en rojo,
  2–4 en naranja (como el mockup). En Pausa se muestran todos apagados.
- Puertos: solo punto verde/rojo + nombre, sin texto de estado.

### 6.3 Settings (⚙) — aprobado tal cual

- MIDI PORTS: *Input name*, *Output name* (texto).
- NUDGE: *Strength while held* (± %).
- *Launch at login* (on/off) — `tauri-plugin-autostart`.
- Botón *Quit Deeboo*.
- Navegación `‹ Settings` para volver.
- Persistencia en JSON en el directorio de configuración de la app.

## 7. Stack técnico (decidido por el usuario)

- **Tauri 2** (Rust) para la app de barra de menú.
- **`midir`** para MIDI (sustituye a mkmidilibrary tras la revisión: más madura, puertos
  virtuales en macOS, timestamps de CoreMIDI). El motor se aísla para poder cambiar la
  librería MIDI sin tocar la lógica.
- UI: **TypeScript + Vite sin framework** y **CSS propio con variables** (tokens de color y
  espaciado), sin framework CSS *(decisión del diseñador: un popover pequeño no lo justifica;
  si la UI crece, Tailwind CSS v4 sería la opción)*.

### 7.1 Estructura propuesta

```
crates/clock-core/   # Rust puro, sin I/O ni MIDI: parser de bytes, BPM, play/pausa,
                     # empuje, contador de beats. Entradas: (timestamp, bytes) y comandos;
                     # salidas: bytes a enviar + pulsos a programar + estado.
src-tauri/           # App: puertos midir, hilos, tray, comandos/eventos Tauri, settings
ui/                  # Frontend TS + Vite
```

- `clock-core` es determinista y se prueba con un reloj simulado: es donde vive todo lo
  crítico.
- Comunicación UI ↔ Rust: comandos Tauri (`play`, `pause`, `nudge_start(dir)`, `nudge_stop`,
  `get_settings`, `save_settings`, `quit`) y un evento de estado (~30 Hz:
  `bpm, beat, playing, in_ok, out_ok`).
- El reenvío de `0xF8` se hace **dentro del callback de entrada** (sin pasar por la UI ni
  por colas), para minimizar latencia.

## 8. Distribución

- `tauri build` genera `.app` y **`.dmg`** (Apple Silicon; universal opcional si es barato).
- **Sin firma ni notarización.** El README explica la instalación: arrastrar a Aplicaciones;
  al abrir, *Ajustes del Sistema → Privacidad y seguridad → Abrir igualmente*; si macOS dice
  que está "dañada": `xattr -dr com.apple.quarantine "/Applications/Deeboo MIDI Sync.app"`.

## 9. Entorno de desarrollo y calidad

### 9.1 Docker

- Imagen de desarrollo (devcontainer + `docker compose`) con Rust fijado
  (`rust-toolchain.toml`), Node LTS, `just` y las dependencias de sistema que Tauri y midir
  necesitan para compilar en Linux (webkit2gtk, ALSA dev).
- Todo lo que no requiere macOS se hace en Docker: formato, lint, tests, `cargo check`.
- **Fuera de Docker (macOS):** ejecutar la app con MIDI real (`just dev`) y generar el DMG.
  Limitación aceptada por el usuario (Docker en Mac es Linux: sin CoreMIDI ni bundles macOS).

### 9.2 Comandos (`justfile`)

`fmt`, `lint`, `test`, `check`, `precommit`, `dev` (macOS), `dmg` (macOS). Las recetas que
deben ir en Docker se ejecutan vía `docker compose run` si no se está ya dentro del contenedor.

### 9.3 Linters y tests

- Rust: `rustfmt`, `clippy -D warnings`, `cargo-deny` (licencias compatibles con GPL-3.0,
  advisories), `cargo test`.
- UI: `tsc --strict`, ESLint, Prettier, Vitest.
- Dockerfile: `hadolint`.

### 9.4 Hooks de git (`lefthook`)

**pre-commit: formato, lint y tests** (requisito explícito del usuario).

### 9.5 CI (GitHub Actions, repo `rolimat/deeboo-midi-sync`)

- **Linux:** fmt, lint, tests y `cargo-deny` dentro de **la misma imagen Docker** que en local.
- **macOS:** clippy + tests del código específico de macOS, `tauri build` → `.dmg` como artefacto.
- **Tag `v*`:** GitHub Release con el `.dmg` adjunto.
- Dependabot para cargo, npm y GitHub Actions.

## 10. Pruebas de aceptación (manuales, en macOS)

1. Con la app abierta, `Deeboo In` y `Deeboo Out` aparecen como puertos MIDI en otros programas.
2. Maestro enviando clock a 124 BPM → la barra muestra `124.0` y el punto verde parpadea.
3. Play → el esclavo arranca y sigue el tempo; Pausa → el esclavo se para.
4. Mantener ▶▶ un segundo → el esclavo se adelanta; al soltar, mantiene el mismo tempo que el
   maestro. Igual para ◀◀ en sentido contrario.
5. Cortar el clock del maestro → In y punto en rojo en menos de 1 s.
6. Renombrar puertos en Settings → los nuevos nombres aparecen en otros programas.
7. Instalar desde el `.dmg` en un Mac limpio siguiendo el README.
8. Stop en el maestro → el esclavo se para y Deeboo pasa a Pausa. Start en el maestro → el
   esclavo **no** arranca hasta pulsar Play en Deeboo.

### 10.1 Medición de jitter (acordado con el usuario)

Criterio principal: funciona bien con el esclavo real del usuario. Además, el plan incluye una
**herramienta de medición** (binario de desarrollo, macOS) que registra los intervalos entre
pulsos `0xF8` recibidos en un puerto virtual y compara:

- conexión directa maestro → herramienta, frente a
- maestro → Deeboo → herramienta, en reposo y durante un empuje sostenido.

Reporta media, desviación típica y p99 del intervalo, y el jitter **añadido** por Deeboo.
Objetivo orientativo: **< 1 ms añadido**. Preferencia del usuario: **evitar bajar a CoreMIDI
directo**; solo se considerará si la medición muestra que midir no alcanza el objetivo, y
sería un cambio aparte.

## 11. Decisiones dejadas al constructor

- Composición exacta del icono de la barra (vinilo + punto) y su legibilidad en claro/oscuro.
- Prioridad/planificación del hilo temporizador de pulsos insertados.
- Formato exacto del fichero de settings y de los logs.
- Build universal (Intel + Apple Silicon) o solo Apple Silicon.
