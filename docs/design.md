# Design

macmetry should feel native, calm and quick to read. This page records the decisions, so a change keeps them.

## Principles

- **A number you can trust.** Values are formatted the way macOS does (disks in 1000s, memory in 1024s). A part the Mac
  does not have is hidden, never shown as zero.
- **Dark first, light chosen.** The light theme is its own set of colours, not an inversion. The window follows the
  system, and the frosted glass follows the window's own theme, so the two always agree.
- **Calm.** Thin lines, a quiet grid, no animation on numbers that change every second. A data-driven animation
  repainted the window constantly and cost 75% of a core before it was removed (now about 18% in a debug build).
- **Colour follows the thing measured**, never its rank: CPU is always blue and memory always violet, on every chart.
  A filter that changes the number of lines does not repaint the others.
- **Never colour alone.** Status colours (calm, busy, hot) always come with a word or a dot, and a chart with two or more
  lines has a legend and a table view.
- **One axis.** Measurements in different units never share a chart.

## Colour

The eight series colours come from a validated categorical palette, in a fixed order, checked for colour-blind
separation and contrast against the surface in each theme:

| Slot | Dark | Light |
|---|---|---|
| 1 blue | `#3987e5` | `#2a78d6` |
| 2 orange | `#d95926` | `#eb6834` |
| 3 aqua | `#199e70` | `#1baf7a` |
| 4 yellow | `#c98500` | `#eda100` |
| 5 magenta | `#d55181` | `#e87ba4` |
| 6 green | `#0ca30c` | `#008300` |
| 7 violet | `#9085e9` | `#6250d6` |
| 8 red | `#e66767` | `#e34948` |

Adjacent colour-blind separation is at least 8.4 (dark) and 9.1 (light) on a scale where 8 is the target, and the normal
vision separation is at least 19. In the light theme three slots (aqua, yellow, magenta) are under 3:1 against white, so
every chart with several lines carries a legend and a table view, as the method requires.

Status colours are separate: good `#3ecf8e`, warning `#f0b13d`, bad `#f2676a` (dark).

## Charts

- Lines are 2 px with round joins; the grid is barely visible; the area fill is a soft gradient.
- A missing value **breaks the line**. It is never drawn as zero.
- Percentages always span 0 to 100, so a quiet Mac does not look busy. Sizes and speeds start at zero and end on a round
  number of their own unit (a 238 MB axis reads 0, 50, 100 MB, not 47.7 MB).
- Hovering shows a crosshair and a tooltip with every series.
- A chart of one measurement over a long range draws the lowest-to-highest band behind the line, so spikes survive
  averaging.

## Language

The interface is in English and Brazilian Portuguese, following the system or the setting. Strings are a flat
dictionary per language; tests check that both have the same keys, that placeholders match, and that every key is used.
