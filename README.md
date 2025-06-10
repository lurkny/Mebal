# Mebal

Mebal is a high-performance screen recording solution designed to provide a streamlined and efficient experience, without the unnecessary bloat found in other recording applications. It's built to deliver high-quality recordings with minimal resource usage.

## Features
- High-quality, low-latency screen recording
- Efficient memory and CPU usage
- Replay buffer for instant save of recent activity
- Modular architecture for easy platform support
- Windows support (Linux and macOS coming soon)

## Dependencies

Mebal requires the FFmpeg shared libraries (DLLs) to be available on your system. These are not bundled with the application. Follow the instructions below to install them for your platform.

### Installing FFmpeg DLLs on Windows

1. **Download FFmpeg Windows Build:**
   - Go to the official [FFmpeg download page](https://ffmpeg.org/download.html) and select a Windows build provider (e.g., [Gyan.dev](https://www.gyan.dev/ffmpeg/builds/)).
   - Download the latest "release full" zip package (e.g., `ffmpeg-release-full.7z` or `.zip`).

2. **Extract the Files:**
   - Extract the downloaded archive to a folder of your choice (e.g., `C:\ffmpeg`).

3. **Locate the DLLs:**
   - Inside the extracted folder, go to `bin/`. You will find files like `avcodec-*.dll`, `avformat-*.dll`, `avutil-*.dll`, `swscale-*.dll`, etc.

4. **Add FFmpeg to your PATH or Copy DLLs:**
   - **Option 1:** Add the `bin/` directory to your system `PATH` environment variable so the DLLs are found automatically.
     - Open the Start Menu, search for "Environment Variables", and edit the `PATH` variable to include the path to your FFmpeg `bin/` folder.
   - **Option 2:** Copy all the DLL files from the `bin/` folder directly into the same directory as your `Mebal.exe` binary.

### Linux and macOS
- On Linux, install FFmpeg using your package manager (e.g., `sudo apt install ffmpeg`).
- On macOS, use Homebrew: `brew install ffmpeg`.
- Ensure the shared libraries are in your system library path (usually handled automatically by the package manager).

---

If you encounter issues, see the [FFmpeg documentation](https://ffmpeg.org/documentation.html) or open an issue on this repository.

## Building
Mebal is a Rust workspace. To build the project, ensure you have Rust and Cargo installed, then run:

```
cargo build --release
```

## Usage
After building, you can run the application:

```
cargo run --release
```

Output files will be saved as `output.mp4` by default. Replay buffer and other advanced features are available via the UI or CLI (see documentation for details).

## Running the GUI (Dioxus)

Mebal's graphical user interface is built with [Dioxus](https://dioxuslabs.com/).

### Option 1: Run the Pre-built Binary (Recommended)
1. Download the latest release for your platform from the [Releases page](https://github.com/yourusername/mebal/releases) (or wherever you distribute binaries).
2. Make sure you have the [FFmpeg DLLs installed](#installing-ffmpeg-dlls-on-windows) (see above).
3. Double-click `Mebal.exe` (Windows) or run the binary for your OS. The Dioxus-powered GUI will launch automatically.

### Option 2: Build and Run from Source (Developers)
If you want to build and run the GUI yourself:


```sh
dx serve
```


If you encounter issues with the GUI, please ensure your graphics drivers are up to date and that you have the required FFmpeg DLLs installed.

## Roadmap
- [x] Windows support
- [ ] Linux support
- [ ] macOS support
- [ ] More advanced configuration options

## Acknowledgements
Mebal uses [FFmpeg](https://ffmpeg.org/) under the LGPL/GPL license for all video and audio encoding/decoding. Huge thanks to the FFmpeg team and contributors for their incredible work on open multimedia technology.

## License
See [LICENSE](LICENSE) for details.

## Note
OSX and Linux will be supported soon, I just wanted to push this code out


