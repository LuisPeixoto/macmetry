# The menu bar

macmetry puts an item in the menu bar with up to four live numbers, for example `CPU 23%  61°`.

## Choosing what it shows

**Settings > Menu bar**: CPU, memory, temperature, power, network and GPU, up to four. A number your Mac cannot
provide is left out, so a desktop without a power reading simply shows less.

## The popover

Click the item for a compact view: CPU, memory, GPU, disk, network, temperature, power and battery with their charts,
the three busiest processes, and a button to open the window. It closes when you click elsewhere.

## Living in the menu bar

- **Keep running when the window closes** (on by default): macmetry keeps recording history and checking alerts.
- **Hide the Dock icon**: macmetry lives only in the menu bar. It needs the menu bar item to be shown, and macmetry
  refuses the combination that would leave no way to open it.
- Clicking the Dock icon, when it is shown, brings the window back.

## Cost

macmetry reads the Mac every second (configurable from 0.5 to 5 seconds). With the window closed it reads only what the
menu bar, history and alerts need and does not read processes.
