# Install

Download or build the binary first. Build steps are in
[Developing](./developing.md).

Prepare an image. A PNG or JPEG both work. The tool scales it to fit and centers
it for the screen. A landscape image close to your screen resolution looks best.

Always run the dry run first. With no `--apply`, the command prints what it
would do and changes nothing.

    bootlogo-forge detect
    bootlogo-forge install --image ./logo.png

When the printed actions look right, run it for real as root:

    sudo bootlogo-forge install --image ./logo.png --apply

You can also switch Plymouth to the BGRT theme at the same time:

    sudo bootlogo-forge install --image ./logo.png --plymouth --apply

Install saves the current boot order and writes a backup file on the ESP before
it changes anything. It creates a new boot entry and puts it first. It never
edits or deletes an existing entry.