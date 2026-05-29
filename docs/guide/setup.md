# Environment Setup

This page explains how to install Mom and verify your environment.

---

## System Requirements

| Component | Minimum |
|---|---|
| OS | Linux (x86_64, ARM64), macOS (Apple Silicon), Windows (x86_64, ARM64) |
| C compiler | `cc` (GCC or Clang) — required for `mom build` |
| Rust + Cargo | 1.78+ — required only when building from source |

---

## Quick Install (Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/jagadeesh32/mom-lang/main/scripts/install.sh | bash
```

Installs to `~/.local/bin/mom`. Add to your PATH if needed:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Verify:

```bash
mom version
# mom 0.3.0
```

### Install a specific version

```bash
curl -fsSL .../install.sh | bash -s -- --version v0.3.0
```

### Install to a custom prefix

```bash
curl -fsSL .../install.sh | bash -s -- --prefix /usr/local
```

---

## Linux — Package Downloads

### Debian / Ubuntu (x86_64)

```bash
curl -fsSL https://github.com/jagadeesh32/mom-lang/releases/latest/download/mom-x86_64.deb -o mom.deb
sudo dpkg -i mom.deb
mom version
```

### Debian / Ubuntu (ARM64)

```bash
curl -fsSL https://github.com/jagadeesh32/mom-lang/releases/latest/download/mom-aarch64.deb -o mom.deb
sudo dpkg -i mom.deb
```

### Red Hat / Fedora / CentOS (x86_64)

```bash
sudo dnf install https://github.com/jagadeesh32/mom-lang/releases/latest/download/mom-x86_64.rpm
```

### Red Hat / Fedora (ARM64)

```bash
sudo dnf install https://github.com/jagadeesh32/mom-lang/releases/latest/download/mom-aarch64.rpm
```

### Universal tarball

```bash
# x86_64
curl -fsSL .../mom-linux-x86_64.tar.gz | tar -xzf - --strip-components=1 -C ~/.local/

# ARM64
curl -fsSL .../mom-linux-aarch64.tar.gz | tar -xzf - --strip-components=1 -C ~/.local/
```

---

## macOS (Apple Silicon M1/M2/M3/M4)

```bash
curl -fsSL .../mom-macos-aarch64.tar.gz | tar -xzf - --strip-components=1 -C ~/.local/
export PATH="$HOME/.local/bin:$PATH"
mom version
```

> Intel Mac is not a packaged target. Use Rosetta 2 (`arch -x86_64 ...`) or build from source.

---

## Windows

### x86_64 (Intel / AMD)

```powershell
Invoke-WebRequest -Uri ".../mom-windows-x86_64.zip" -OutFile mom.zip
Expand-Archive mom.zip -DestinationPath "$env:LOCALAPPDATA\mom" -Force

$binDir = "$env:LOCALAPPDATA\mom\mom-windows-x86_64"
[Environment]::SetEnvironmentVariable("PATH", "$binDir;$([Environment]::GetEnvironmentVariable('PATH','User'))", "User")
```

### ARM64 (Snapdragon / Copilot+ PCs)

```powershell
Invoke-WebRequest -Uri ".../mom-windows-aarch64.zip" -OutFile mom.zip
Expand-Archive mom.zip -DestinationPath "$env:LOCALAPPDATA\mom" -Force

$binDir = "$env:LOCALAPPDATA\mom\mom-windows-aarch64"
[Environment]::SetEnvironmentVariable("PATH", "$binDir;$([Environment]::GetEnvironmentVariable('PATH','User'))", "User")
```

Open a new terminal and run:

```cmd
mom version
```

---

## Build from Source

Requires **Rust 1.78+** and `cargo`.

```bash
git clone https://github.com/jagadeesh32/mom-lang.git
cd mom
cargo build --release
./target/release/mom version
```

### Verify self-hosting

```bash
# Compile the mom-in-mom compiler with itself
./target/release/mom selfhost compiler/src/main.mom -o mom-stage1

# Run a program with the self-hosted binary
MOM_INPUT=examples/hello.mom MOM_OUTPUT=/tmp/hello.c ./mom-stage1
gcc -std=c99 -I compiler compiler/runtime.c /tmp/hello.c -o hello
./hello
# Hello, world!
```

---

## IDE Integration

### VS Code

Install the **Mom Language** extension from the VS Code marketplace. It uses `mom lsp` under the hood and provides:

- Syntax highlighting
- Type-on-hover
- Go-to-definition
- Inline diagnostics
- Format on save

### JetBrains (IntelliJ, CLion, etc.)

Install the **Mom** plugin from the JetBrains Marketplace.

### Neovim / Vim

Configure `mom lsp` as an LSP server via `nvim-lspconfig` or `vim-lsp`:

```lua
require'lspconfig'.mom.setup{}
```

---

## Checking Your Installation

```bash
mom version            # prints version
mom help               # prints all commands
mom run examples/hello.mom     # run a sample program
```

---

## Uninstall

### From tarball / install.sh

```bash
rm ~/.local/bin/mom
```

### From .deb

```bash
sudo dpkg -r mom
```

### From .rpm

```bash
sudo dnf remove mom
```
