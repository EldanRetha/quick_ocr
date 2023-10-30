# Overview

There are numerous OCR options that are useful for various applications. However,
the objective of quickocr is to provide a convenient interface for performing OCR on
an arbitrary area of the screen.

The primary motivation for this was to allow me to look up kanji without having to handwrite
it for detection.

To accomplish this I had the idea of implemnting something akin to the Windows Snipping Tool,
but that instead of capturing a screenshot would copy the text to your keyboard.

# Implementation

This is accomplished by the following steps:

1. Capture your screens
2. Create a borderless fullscreen window for each screen
3. Set each window to the associated captured screen image
4. Allow user to select a section by drawing a red square
5. Capture sub-image based on mouse coordinates
6. Pass sub-image to Tesseract
7. Copy results 

# Installation

1. Run `cargo build --release`
2. Install Tesseract (instructions TBD)
3. Create a shortcut to the application as the first applications on your toolbar so
you can hit `Windows/Super + 1` to run it

# TODO

1. Clean up messy code
2. Allow other orientations/languages (probably via command line params)
3. Migrate to a crate that uses a static Tesseract library rather than the executable