# Synesthesia

> "*The word “synesthesia” comes from the Greek words: “synth” (which means “together”) and “ethesia” (which means “perception). Synesthetes can often “see” music as colors when they hear it...*"

## Description

This project is meant to become a fully featured audio player, putting an emphasis on being extremely customizable through scripts and an interface that lets users mix sound and visuals easily and intuitively.

As of now, it only features a really basic demo using the Fast Fourier Transform to animate a few cubes, but will quickly grow into a more fully fledged application!

## Demo

![Demo](demo.gif)

## Table of contents

- [Synesthesia](#synesthesia)
- [Demo](#demo)
- [Table of contents](#table-of-contents)
- [Installation](#installation)
- [Usage](#usage)
- [Development](#development)
- [License](#license)

## Installation

[Back to top](#table-of-contents)

If you wish to run the demo, your only option is compiling from source, by simply cloning this repository anywhere on your computer and running `cd synesthesia && cargo build --release`.

You will then find the executable in the `target/release` folder.

## Usage

[Back to top](#table-of-contents)

The demo can be used by passing a path to an audio file as an argument.

The file will then be loaded up from the disk (this will change in later versions for obvious reasons) and the demo will play!

You can then make it fullscreen by pressing F11, or quit by pressing Escape.

## Development

[Back to top](#table-of-contents)

For now, the project is under HEAVY developement. As of the time of writing this paragraph, it simply features the demo you can see on top of this README, as well as a bunch of bugs.

The only way you can customize this is through the source code, there are problems with loading sound files, and the whole thing is far from scalable at the moment.

This will change later on when I'll add:

- Scripting, to create scenes easily
- A more complete version of the rendering system
- More options for audio sources (microphone, other programs, etc)

## License

[Back to top](#table-of-contents)

If you don't want to go through all of the text, hsere's a quick recap:

- You cannot distribute a closed source program featuring code from this project...
- ...but you can do close to anything apart from that!

The more verbose (but accurate) version can be found here: [GNU GPL v3](https://opensource.org/licenses/GPL-3.0)
