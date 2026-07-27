# BoxPacker

BoxPacker finds a practical way to fit rectangular items into one or more
rectangular containers.

Give it the dimensions of your boxes and items in a JSON file. BoxPacker can
rotate items in 90-degree increments, searches for a good arrangement, and
creates:

- a JSON file containing every placement and any items that did not fit; and
- an interactive 3D HTML report that lets you inspect the arrangement.

It is useful for planning moving boxes, storage bins, shipping cartons, or any
similar problem where everything can be represented as a rectangular cuboid.

> BoxPacker is a geometric planning tool. It does not account for weight,
> fragility, balance, stacking strength, or required item orientation.

## Quick start

### 1. Create an input file

Save the following as `input.json`:

```json
{
  "containers": [
    {
      "name": "Large box",
      "width": 60,
      "length": 40,
      "height": 40
    }
  ],
  "contents": [
    {
      "name": "Books",
      "width": 30,
      "length": 20,
      "height": 15
    },
    {
      "name": "Lamp",
      "width": 18.5,
      "length": 18.5,
      "height": 35
    },
    {
      "name": "Shoes",
      "width": 32,
      "length": 20,
      "height": 12
    }
  ]
}
```

Use the same unit for every dimension—centimetres, inches, or anything else.
Dimensions must be greater than zero and may have at most one decimal place.

### 2. Run BoxPacker

With [Nix](https://nixos.org/) installed, run BoxPacker directly from GitHub:

```sh
nix run github:oDHAOSo/boxpacker -- \
  --input input.json \
  --output packing.json
```

BoxPacker uses the `balanced` search preset by default. When it finishes, it
prints how many items were packed and writes two files:

- `packing.json` — placement coordinates, oriented dimensions, and unplaced
  items;
- `packing.html` — an interactive 3D view of the same result.

Open `packing.html` in a web browser to explore the proposed layout. The report
starts with one container in focus: choose another from the container list, use
**Show All Containers** for a gallery of the full result, or switch the focused
container to **Layer View** and build up its contents from bottom to top. Boxes
for the current step retain their full dimensions while earlier boxes remain as
faint context. Layer View can be orbited and includes a colored axis guide;
**Reset Top View** restores the overhead orientation. The numbered contents
list follows the suggested bottom-up placement order. The report needs an
internet connection to load its 3D viewer libraries.

## Install or build

### Download a release

Non-Nix users can download a prebuilt archive from the
[latest GitHub release](https://github.com/oDHAOSo/boxpacker/releases/latest):

| Platform | Archive suffix |
| --- | --- |
| ARM64 macOS | `aarch64-apple-darwin.tar.gz` |
| ARM64 Linux | `aarch64-unknown-linux-gnu.tar.gz` |
| x86-64 Linux | `x86_64-unknown-linux-gnu.tar.gz` |
| ARM64 Windows | `aarch64-pc-windows-msvc.exe` |
| x86-64 Windows | `x86_64-pc-windows-msvc.exe` |

Each download has an accompanying `.sha256` checksum file. macOS and Linux
builds are compressed archives; Windows builds are ready-to-run `.exe` files.

On macOS or Linux:

```sh
tar -xzf boxpacker-v0.2.0-x86_64-unknown-linux-gnu.tar.gz
mkdir -p ~/.local/bin
install -m 0755 boxpacker ~/.local/bin/boxpacker
boxpacker --version
```

On Windows, run the downloaded executable directly:

```powershell
.\boxpacker-v0.2.0-x86_64-pc-windows-msvc.exe --version
```

Move it to a directory on your `PATH` and optionally rename it to
`boxpacker.exe`. Replace the example version and platform in these commands
with the asset you downloaded.

### Add to a Nix configuration

The Nix package supports ARM64 macOS, ARM64 Linux, and x86-64 Linux.

First, add BoxPacker to the `inputs` in your system's `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    boxpacker.url = "github:oDHAOSo/boxpacker";
  };

  # ...
}
```

Capture the complete input set in your flake's `outputs`, then pass it to your
NixOS or nix-darwin modules when constructing the system:

```nix
outputs = inputs@{ nixpkgs, ... }: {
  nixosConfigurations.your-host = nixpkgs.lib.nixosSystem {
    # Keep your existing system and modules here.
    system = "x86_64-linux";
    modules = [ ./configuration.nix ];
    specialArgs = { inherit inputs; };
  };
};
```

For nix-darwin, add the same `specialArgs` attribute to your existing
`darwinSystem` definition.

Then add BoxPacker to `configuration.nix`:

```nix
{ inputs, pkgs, ... }:

{
  environment.systemPackages = [
    inputs.boxpacker.packages.${pkgs.stdenv.hostPlatform.system}.default
  ];
}
```

Home Manager users can put the same package expression in `home.packages`
instead. Make `inputs` available to the Home Manager module with
`extraSpecialArgs = { inherit inputs; };` when needed.

BoxPacker will update with the rest of the system after updating the flake lock
file and rebuilding.

### Install into a Nix profile

If you do not manage your packages through a Nix configuration, install it
imperatively instead:

```sh
nix profile install github:oDHAOSo/boxpacker
boxpacker --help
```

### Build from source

BoxPacker uses the stable Rust toolchain:

```sh
git clone https://github.com/oDHAOSo/boxpacker.git
cd boxpacker
cargo build --locked --release
./target/release/boxpacker --help
```

Run it from the source tree with:

```sh
cargo run --locked --release -- \
  --input input.json \
  --output packing.json
```

## Command-line options

```text
boxpacker [OPTIONS]

  -i, --input <FILE>          Input JSON file [default: input.json]
  -o, --output <FILE>         Output JSON file [default: output.json]
      --preset <PRESET>       Search effort: fast, balanced, or thorough
      --time-limit <SECONDS>  Override the preset's time limit
      --seed <SEED>           Seed for reproducible search [default: 0]
      --threads <COUNT>       Maximum solver threads [default: 1]
  -h, --help                  Show help
  -V, --version               Show version
```

The presets trade search effort for time:

| Preset | Default time limit | When to use it |
| --- | ---: | --- |
| `fast` | 1 second | Quick estimates and small inputs |
| `balanced` | 10 seconds | The recommended starting point |
| `thorough` | 30 seconds | More search when packing quality matters |

The time limit is a ceiling, not a target; BoxPacker may finish much sooner.
Increasing `--threads` lets independent search work run concurrently.

## How results are chosen

Every proposed layout is checked for container bounds, valid rotation,
overlap, and exactly-once item coverage. Among valid layouts, BoxPacker first
maximizes the total packed volume, then the number of packed items, and then
prefers layouts using fewer containers.

BoxPacker is a deterministic heuristic, not an optimality prover. With the same
input, preset, seed, and thread count, it normally produces the same layout.
A tight wall-clock time limit can make results vary between machines.

See [the usage and solver guide](docs/usage.md) for the full objective,
algorithm, status meanings, reproducibility details, and current limitations.

## Input and output at a glance

The top-level input fields are:

| Field | Meaning |
| --- | --- |
| `containers` | The available rectangular spaces |
| `contents` | The rectangular items to place |

Each entry has a `name`, `width`, `length`, and `height`. Items may be rotated;
there is currently no way to restrict an item's orientation.

The output preserves each container's dimensions and adds `placed_items`.
Every placed item includes:

- `coords.x`, `coords.y`, and `coords.z` for its position;
- `coords.w`, `coords.l`, and `coords.h` for its dimensions after rotation;
- a display `color` used by the HTML report.

Items that cannot be placed appear in `unplaced_items`.

## Development

The reproducible development environment is managed by
[devenv](https://devenv.sh/):

```sh
devenv shell
devenv test
```

`devenv test` checks formatting, runs Clippy with warnings denied, and runs all
test targets. The same checks run in CI on ARM64 macOS, ARM64 and x86-64 Linux,
and ARM64 and x86-64 Windows.

Design background and benchmark evidence live in
[the solver decision record](docs/decisions/0001-solver-backend.md) and
[the solver bake-off](docs/bakeoff/M2.5.md).

### Creating a release

Start from a clean `main` branch that exactly matches `origin/main`, then run:

```sh
devenv tasks run boxpacker:release --input version=0.2.0
```

Replace `0.2.0` with the new `MAJOR.MINOR.PATCH` version. The task updates
`Cargo.toml` and `Cargo.lock`, runs the full format, lint, test, and release
build checks, confirms the binary reports the requested version, creates an
annotated `v0.2.0` tag, and atomically pushes the release commit and tag.

The tag starts the GitHub release workflow. It verifies the version again,
builds downloadable assets for every supported platform, adds SHA-256 checksums
to a draft release, and publishes the release only after all builds succeed. If
a platform fails, the incomplete release remains a draft.
