# Noct Console Theme

Noct normally uses full RGB themes on terminals that support them. Physical
system consoles are a different environment. They usually have a much smaller
ANSI 16-color palette, so Noct keeps a separate theme for them.

The console theme is called:

```text
console_ansi16.toml
```

When installed for your user, Noct looks for it here:

```text
~/.config/noct/themes/console/console_ansi16.toml
```

You do not select this theme in `noct.toml`.

Noct uses it automatically whenever ANSI16 console colors are needed. Your
normal RGB theme remains selected separately with the `theme =` setting in
`noct.toml`.


## Keep the filename

Unlike Noct's regular themes, the console theme has a fixed filename:

```text
console_ansi16.toml
```

You are free to edit the colors inside it, but Noct looks specifically for a
file with that name.

If you rename it, Noct will no longer use it as the active console theme.

This is different from the normal RGB themes, where several theme files can
live together and you select one of them by name in `noct.toml`.


## Changing the console theme

The easiest way to make the console colors your own is simply to edit
`console_ansi16.toml`.

Normal Noct themes use RGB colors such as:

```toml
heading = "#d8d2c4"
```

The console theme instead uses names from the ANSI 16-color palette:

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

`light_gray` and `white` are deliberately different. `light_gray` uses the
regular light-gray ANSI slot, while `white` uses bright white.

The console palette is much smaller than the RGB palette used by normal Noct
themes. Because of that, console themes often look better when a few colors are
reused consistently instead of trying to give every kind of information its
own color.


## Keeping several console themes

Noct uses only one active console theme at a time, and the active file must
always be named:

```text
console_ansi16.toml
```

You can still keep your own alternatives.

For example:

```text
console_ansi16.blue.toml
console_ansi16.green.toml
console_ansi16.oldschool.toml
```

When you want to use one of them, copy its contents to:

```text
console_ansi16.toml
```

Noct will then use that version the next time ANSI16 console colors are needed.

The important part is simply that the active file keeps the fixed
`console_ansi16.toml` name.


## You do not need to define everything

Every entry in the console theme is optional.

If an entry is missing, Noct keeps its built-in ANSI16 color for that part of
the display.

This means you can create a very small custom theme if you only want to change
a few things.

For example, you could change only a handful of report colors and leave the
rest of the file untouched.


## Invalid colors

If a color name is not recognized, Noct prints a warning and keeps the built-in
fallback color for that entry.

A mistake in one color therefore does not make the rest of the theme unusable.


## If the file is missing

Noct has a complete ANSI16 palette built into the program.

If `console_ansi16.toml` cannot be found, Noct simply uses those built-in
colors instead.

The external file exists so that the console palette can be changed without
recompiling Noct. It is an override, not a requirement for console support.


## Normal themes and the console theme

Normal RGB themes and the ANSI16 console theme are two separate parts of
Noct's theming system.

Normal themes are selected by name:

```toml
theme = "verdant_mocha"
```

and live among Noct's regular theme files.

The console theme is not selected by name. Noct uses
`console_ansi16.toml` automatically when ANSI16 output is required.

This means you can have one appearance in your normal terminal and another on
the physical console without changing your configuration each time.