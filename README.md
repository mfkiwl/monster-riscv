# monster
Monster is a symbolic execution engine for 64-bit RISC-V code

# Toolchain setup
## Install rust
1. Bootstrap rust
```
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
2. Install Rustfmt (formatter) and Clippy (linter)
```
$ rustup component add rustfmt
$ rustup component add clippy
```
3. Add cargo to your $PATH
```
$ echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```
4. Install tool for cross compilation
```
$ cargo install cross
```

## Install boolector

In order to generate a smt formula out of an `Formula` graph boolector
is used. To successfully build monster on Linux it is necessary to
provide the path to `boolector.so` for example via the environment
variable `LD_LIBRARY_PATH`.

5. Build boolectors shared object file

```sh
git clone https://github.com/Boolector/boolector.git
cd boolector
contrib/setup-lingeling.sh
contrib/setup-btor2tools.sh
./configure.sh --shared
cd build
make
```

## Docker and llvm
### Debian based
6. Install docker (needed by cross) with [this installation guide](https://docs.docker.com/engine/install/debian/)
7. Make sure you have a recent version of clang/llvm (>= v9) installed:
```
$ apt install llvm
```

### Mac
6. Install docker (needed by cross) with [this installation guide](https://docs.docker.com/docker-for-mac/install/)
7. Make sure you have a recent version of clang/llvm (>= v9) installed:
```
$ brew install llvm
```

## Build and test
8. Test your toolchain setup by compiling monster:

### Mac
```
$ DYLD_LIBRARY_PATH=<path_to_boolector>/build/lib cargo build
```

### Debian based
```
$ LD_LIBRARY_PATH=<path_to_boolector>/build/lib cargo build
```

9. Execute tests:
```
$ cargo test
```
