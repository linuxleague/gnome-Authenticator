<a href="https://flathub.org/apps/details/com.belmoussaoui.Authenticator">
<img src="https://flathub.org/assets/badges/flathub-badge-i-en.png" width="190px" />
</a>

# Authenticator

<img src="https://gitlab.gnome.org/bilelmoussaoui/authenticator/raw/master/data/icons/com.belmoussaoui.Authenticator.svg" width="128px" height="128px" />

<p>Generate Two-Factor Codes</p>

## Screenshots

![screenshot](data/screenshots/screenshot1.png)

## Features

- Time-based/Counter-based/Steam methods support
- SHA-1/SHA-256/SHA-512 algorithms support
- QR code scanner using a camera or from a screenshot
- Lock the application with a password
- Beautiful UI
- Backup/Restore from/into known applications like FreeOTP+, andOTP

## Getting in touch

If you have any questions regarding the use or development of Authenticator, please join us on our [#authenticator:gnome.org](https://matrix.to/#/#authenticator:gnome.org) channel.

## Known issue

- If the application crashes once you try to use the camera scanning feature while running the application under Wayland with an Intel GPU: The issue is caused by the driver and can be worked around by forcing the application to run under X11.

You can override the permissions if you're using Flatpak with
```
flatpak override com.belmoussaoui.Authenticator --nosocket=fallback-x11 --nosocket=wayland --socket=x11
```

See the [mesa issue](https://gitlab.freedesktop.org/mesa/mesa/-/issues/3029) for more details.

## Hack on Authenticator

To build the development version of Authenticator and hack on the code
see the [general guide](https://wiki.gnome.org/Newcomers/BuildProject)
for building GNOME apps with Flatpak and GNOME Builder.

## Credits

- We ship a database of providers based on [twofactorauth](https://github.com/2factorauth/twofactorauth), by the 2factorauth team
