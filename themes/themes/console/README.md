# Noct Console Themes

**Noct** normally uses full RGB themes on terminals that support them. Physical system consoles are a different environment. They usually provide a much smaller ANSI 16-color palette, and the exact appearance and capabilities of that palette can differ between operating systems.

For this reason, **Noct** keeps its console themes separate from its normal RGB themes and can automatically select an operating-system-specific console theme when one is available.

## Automatic Console Theme Selection

When **Noct** detects a supported physical system console, it switches automatically to ANSI16 console colors. You do not select the console theme with the `theme` setting in `noct.toml`.

**Noct** supports operating-system-specific console theme files with names such as:

```text
console_ansi16_freebsd.toml
console_ansi16_netbsd.toml
console_ansi16_openbsd.toml
console_ansi16_dragonfly.toml
```

These allow the palette to be adjusted for differences between system consoles.

Packaged installations can provide the appropriate operating-system-specific theme in **Noct's** system theme directory. **Noct** selects it automatically when ANSI16 console output is required.

If no suitable external console theme is available, **Noct** has a complete built-in ANSI16 palette that it can use as a fallback.

## Personal Console Theme

A personal console theme can be stored at:

```text
~/.config/noct/themes/console/console_ansi16.toml
```

This file acts as the user's console-theme override and is selected automatically when ANSI16 console colors are needed.

Unlike normal RGB themes, it is **not** selected by name with the `theme` setting in `noct.toml`.

## Keep the Personal Override Filename

The active personal console theme has a fixed filename:

```text
console_ansi16.toml
```

You are free to edit the colors inside it, but **Noct** looks specifically for a personal override with that name. So the name must always be `console_ansi16.toml`.

This differs from normal RGB themes, where several theme files can live together and one is selected by name in `noct.toml`.

## Changing the Console Theme

Normal **Noct** themes use RGB colors such as:

```toml
heading = "#d8d2c4"
```

Console themes instead use names from the ANSI 16-color palette:

```toml
heading = "white"
label = "bright_cyan"
muted = "dark_gray"
```

The available foreground colors are:

```text
black
red
green
yellow
blue
magenta
cyan
light_gray

dark_gray
bright_red
bright_green
bright_yellow
bright_blue
bright_magenta
bright_cyan
white
```

`light_gray` and `white` are deliberately different. `light_gray` uses the regular light-gray ANSI slot, while `white` represents bright white.

The console palette is much smaller than the RGB palette used by normal **Noct** themes. Because of that, console themes often look better when a few colors are reused consistently instead of trying to give every kind of information its own color.

## DragonFly BSD Console Colors

DragonFly BSD's physical system console handles bright ANSI colors differently from the other consoles currently supported by **Noct**.

For bright foreground colors, **Noct's** DragonFly build uses the console's bold/intensity form of the standard ANSI colors rather than the modern bright foreground range. This allows console-theme names such as:

```text
bright_red
bright_green
bright_yellow
bright_blue
bright_magenta
bright_cyan
```

to remain usable normally in a DragonFly console theme.

Bright ANSI background colors are not reliably supported by the DragonFly system console. The supplied DragonFly console theme therefore uses colors from the standard background range where necessary. In particular, its header uses a standard cyan background rather than `bright_cyan`.

This handling is automatic when using the supplied DragonFly package and console theme.

## Keeping Several Personal Console Themes

**Noct** uses only one personal console-theme override at a time, and the active file must be named:

```text
console_ansi16.toml
```

You can still keep your own alternatives. For example:

```text
console_ansi16.blue.toml
console_ansi16.green.toml
console_ansi16.oldschool.toml
```

When you want to use one of them, copy its contents to:

```text
console_ansi16.toml
```

**Noct** will then use that version the next time ANSI16 console colors are needed.

The important part is simply that the active personal override keeps the fixed `console_ansi16.toml` name.

## You Do Not Need to Define Everything

Every entry in a console theme is optional.

If an entry is missing, **Noct** keeps its built-in ANSI16 color for that part of the display.

This means you can create a very small personal console theme if you only want to change a few things. For example, you can change only a handful of report colors and leave everything else unspecified.

## Invalid Colors

If a color name is not recognized, **Noct** prints a warning and keeps the built-in fallback color for that entry.

A mistake in one color therefore does not make the rest of the theme unusable.

## If No External Theme Is Available

**Noct** has a complete ANSI16 palette built into the program.

When ANSI16 console colors are required, **Noct** can use a personal console-theme override or an appropriate installed console theme. If neither is available, it uses its built-in ANSI16 palette.

The external theme files therefore allow the console palette to be customized or adapted to a particular operating system without making them a requirement for console support.

## Normal Themes and Console Themes

Normal RGB themes and ANSI16 console themes are two separate parts of **Noct's** theming system.

Normal themes are selected by name:

```toml
theme = "verdant_mocha"
```

and live among **Noct's** regular theme files.

Console themes are selected automatically when ANSI16 console output is required. This means you can have one appearance in your normal terminal and another on the physical system console without changing your configuration each time.