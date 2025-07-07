# hdiff-apply

## Features:
- Optional client integrity verification by file size and MD5
- Hdiff version validation
- Support for sequential updates
- Parallelized patching process
- Parallelized verification process

## Requirements:
- [Nightly rust](https://www.rust-lang.org/tools/install) for compiling

## How to use (easiest way):
1. Download the latest version from [releases](https://github.com/nie4/hdiff-apply/releases)
2. Move `hdiff-apply.exe` to the same folder where the game is located
3. Put the hdiff update package to SR folder (without extracting)
4. Run `hdiff-apply.exe` and wait for it to finish

## CLI usage:
```
Usage: hdiff-apply.exe [GAME_PATH]

Arguments:
  [GAME_PATH]
```

## Compiling:
```bash
cargo build -r
```

## Credits:
- [HDiffPatch](https://github.com/sisong/HDiffPatch) for the patching utility (`bin/hpatchz.exe`)
- [7-Zip](https://7-zip.org/) for file archive utility (`bin/7z.exe`)
