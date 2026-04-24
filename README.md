# hdiff-apply

## Features
- Support for HDiff and LDiff
- Sequential updates
- Parallelized patching process
- Safe patching: Game files remain unchanged if patching fails

## How to use (easiest way)
1. Download the latest version from [releases](https://github.com/nie4/hdiff-apply/releases)
2. Place `hdiff-apply.exe` in your game installation directory
3. Put your patch archive(s) in the same folder (do not extract)
4. Run `hdiff-apply.exe` and follow the prompts

## CLI usage
```
Usage: hdiff-apply.exe [OPTIONS]

Options:
  -g, --game-path <PATH>      Game installation directory [default: .]
  -a, --archives-path <PATH>  Directory containing patch archives (defaults to game_path)
  -h, --help                  Print help
  -V, --version               Print version

EXAMPLES:
  # Apply patches from current directory
  hdiff-apply

  # Specify game installation path
  hdiff-apply -g "C:\Games\GameName"

  # Patch archives in different directory
  hdiff-apply -g "C:\Games\GameName" -a "D:\Downloads\patches"
```

## Building from Source

### Prerequisites
- [Rust toolchain](https://www.rust-lang.org/tools/install) (nightly)

### Compilation
```bash
git clone https://github.com/nie4/hdiff-apply.git
cd hdiff-apply
cargo build --release
```

## Credits
- [HDiffPatch](https://github.com/sisong/HDiffPatch) for the patching utility (`hpatchz/bin/hpatchz.exe`)
- [7-Zip](https://7-zip.org/) for file archive utility (`seven-zip/bin/7z.exe`)
- [SophonPatcher](https://github.com/WatchAndyTW/SophonPatcher/) for original ldiff manifest proto
- [Hi3Helper.Sophon](https://github.com/CollapseLauncher/Hi3Helper.Sophon) for updated sophon proto

## Issues & Contributions
Found a bug or want to contribute? Please open an issue or pull request on [GitHub](https://github.com/nie4/hdiff-apply).