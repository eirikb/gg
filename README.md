# `gg.cmd`

[![gg.cmd](https://github.com/eirikb/gg/actions/workflows/gg.yml/badge.svg)](https://github.com/eirikb/gg/actions/workflows/gg.yml)
[![Release](https://badgen.net/github/release/eirikb/gg)](https://github.com/eirikb/gg/releases/latest/download/gg.cmd)

[**[Changelog]**](https://github.com/eirikb/gg/releases)
[**[Download]**](https://github.com/eirikb/gg/releases/latest/download/gg.cmd)

![Image](https://github.com/user-attachments/assets/35d6bc4f-ea3b-4673-a874-45703c4de1d8)

![Image](https://github.com/user-attachments/assets/93876050-9c28-4389-a77c-5a88f5af2811)

`gg.cmd` is a cross-platform and cross-architecture command-line interface (CLI) that acts as an executable wrapper for
various tools such as Gradle, JDK/JVM, Node.js, and Java. **It requires zero external dependencies** - works on plain
Alpine and Ubuntu containers without curl/wget or updated CA certificates (all networking is built-in). Similar in
functionality to gradlew (without need for JDK initially installed).

Install with bash (wget):
> wget ggcmd.io/gg.cmd

Install with bash (curl):
> curl -L ggcmd.io > gg.cmd

Install with PowerShell:
> wget ggcmd.io -OutFile gg.cmd

or

[Download the latest release](https://github.com/eirikb/gg/releases/latest/download/gg.cmd)

**Install?**  
The concept involves placing a copy of `gg.cmd` in the root directory of your project.  
This is similar to what you would do with `gradlew` or `mvnw`, except this method is applicable to multiple tools.  
As a result, your colleagues would not have to install anything on their host machines.

## Features

- **Zero dependencies** - Works on minimal containers (Alpine, Ubuntu) without curl/wget or CA certificates
- **Configuration system** - Define version requirements and command aliases via `gg.toml` files
- Simplify the management of other executables in your project
- Automatically detect and execute the required executable version using project configuration files (such
  as `package.json` for Node.js projects)
- Support for chaining multiple executables (e.g. `gradle@6:java@17`)
- Cross-platform compatibility (Windows, macOS, and Linux)
- Cross-architecture compatibility (x86_64 and ARM)
- Fast and lightweight

By default, installs tools in a global cache directory (`$HOME/.cache/gg` on Unix, `%UserProfile%\.cache\gg` on
Windows).
Use the `-l` flag to use a local cache (`.cache/gg` in current directory) instead.
Adds every dependency into `PATH` before executing.

## Configuration

`gg.cmd` supports project-specific configuration via a `gg.toml` file. This allows you to define version requirements
and command aliases for your project.

Note, this is not a full task runner, if you need a full task runner, consider using [just](https://just.systems/),

### Creating a Configuration File

Create a `gg.toml` configuration file in your project root:

```bash
./gg.cmd config init
```

This creates a `gg.toml` file with the following structure:

```toml
# gg configuration file
# See https://github.com/eirikb/gg for more information

[dependencies]
# Define version requirements for tools
# Examples:
# node = "^18.0.0"
# java = "17"
# gradle = "~7.6.0"

[aliases]
# Define command shortcuts
# Examples:
# build = "gradle clean build"
# serve = "node@18 server.js"
# test = "npm test"
```

### Configuration Features

**Version Dependencies**: Ensure team members use compatible tool versions

```toml
[dependencies]
node = "^18.0.0"      # Allow 18.x.x versions
java = "17"           # Require exactly Java 17
gradle = "~7.6.0"     # Allow 7.6.x versions
```

**Command Aliases**: Create shortcuts for common commands

```toml
[aliases]
build = "gradle clean build"
serve = "node@18 server.js --port 3000"
test = "npm test"
gen = "dart scripts/generate.dart"
```

### Using Aliases

Once defined, aliases can be used directly:

```bash
./gg.cmd build           # Expands to: ./gg.cmd gradle clean build
./gg.cmd serve --dev     # Expands to: ./gg.cmd node@18 server.js --port 3000 --dev  
./gg.cmd test --coverage # Expands to: ./gg.cmd npm test --coverage
```

Aliases support `&&` for sequential execution:

```toml
[aliases]
build-and-test = "gradle clean build && npm test"
```

### Viewing Configuration

View your current configuration:

```bash
./gg.cmd config show
```

This will display the configuration file location, its contents, and parsed aliases/dependencies.

## Usage

Using `gg.cmd` is easy. Simply place the executable in the root of your project and run it with the `gg.cmd` command
followed
by the desired executable and its required dependencies:

```bash
./gg.cmd [gg options] <executable name>@<version><+include_tags><-exclude_tags>:<dependent executable name>@<version><+include_tags><-exclude_tags> [executable arguments]
```

### Example

```bash
sh gg.cmd npm install
```

```
Usage: ./gg.cmd [options] <executable name>@<version>:<dependent executable name>@<version> [program arguments]

Options:
    -l              Use local cache (.cache/gg) instead of global cache
    -v              Info output
    -vv             Debug output
    -vvv            Trace output
    -w              Even more output
    -h, --help      Print help
    -V, --version   Print version
    --os <OS>       Override target OS (windows, linux, mac)
    --arch <ARCH>   Override target architecture (x86_64, arm64, armv7)

Built in commands:
    update          Check for updates for all tools (including gg)
    update -u       Update all tools that have updates available
    update <tool>   Check for updates for specific tool (e.g., update flutter, update gg)
    update <tool> -u Update specific tool (e.g., update flutter -u, update gg -u)
    update <tool> -u -f Force update specific tool even if up to date (e.g., update gg -u -f)
    help            Print help
    tools           List all available tools
    clean-cache     Clean cache (prompts for confirmation)
    config init     Create a new gg.toml configuration file
    config show     Show current configuration

Update options:
    -u              Actually perform the update (vs just checking)
    -f              Force re-download even if already up to date (use with -u)
    --major         Include major version updates (default: skip major versions)

Version syntax:
    @X              Any X.y.z version (e.g. node@14 for any Node.js 14.x.y)
    @X.Y            Any X.Y.z patch version (e.g. node@14.17 for any Node.js 14.17.z)
    @X.Y.Z          Exactly X.Y.Z version (e.g. node@14.17.0 for exactly Node.js 14.17.0)
    @^X.Y.Z         X.Y.Z or any compatible newer version (caret prefix)
    @~X.Y.Z         X.Y.Z or any newer patch version (tilde prefix)
    @=X.Y.Z         Exactly X.Y.Z version (equals prefix, same as X.Y.Z without prefix)

Examples:
    ./gg.cmd node
    ./gg.cmd -l node                                      (use local cache)
    ./gg.cmd gradle@6:java@17 clean build
    ./gg.cmd -l gradle@6:java@17 clean build             (use local cache)
    ./gg.cmd node@10 -e 'console.log(1)'
    ./gg.cmd node@14.17.0 -v
    ./gg.cmd -vv -w npm@14 start
    ./gg.cmd java@-jdk+jre -version
    ./gg.cmd jbang hello.java
    ./gg.cmd bld version
    ./gg.cmd maven compile
    ./gg.cmd run:java@17 soapui
    ./gg.cmd run:java@14 env
    ./gg.cmd update
    ./gg.cmd gh/cli/cli --version
    ./gg.cmd --os windows --arch x86_64 deno --version    (test Windows Deno on Linux)
    ./gg.cmd --os mac deno --help                         (test macOS Deno from anywhere)

Example tools:
    node        Node.js JavaScript runtime (npm, npx will also work)
    java        Java runtime and development kit
    gradle      Gradle build automation tool
    go          Go programming language
    flutter     Flutter SDK (dart will also work)

Run 'gg tools' to see all available tools with descriptions

GitHub repos can be accessed directly:
    gh/<owner>/<repo>    Any GitHub release (e.g. gh/cli/cli)

Available tags by tools:
    java: +jdk, +jre, +lts, +sts, +mts, +ea, +ga, +headless, +headfull, +fx, +normal, +hotspot (defaults: +jdk, +ga)
    node: +lts
    go: +beta (excluded by default)
    openapi: +beta (excluded by default)
```

## Node

Version from:

* `engines` in `package.json`
* Contents of `.nvmrc`

## Gradle

Version from:

* `distributionUrl` in `gradle/wrapper/gradle-wrapper.properties`
* `distributionUrl` in `gradle.properties`

Download URL from:

* `distributionUrl` in `gradle/wrapper/gradle-wrapper.properties`
* `distributionUrl` in `gradle.properties`

## JBang

The Java version is read from the JBang script using the [
`//JAVA magic comment`](https://www.jbang.dev/documentation/guide/latest/javaversions.html).

## Java

Version from:

* `jdkVersion` in `gradle/wrapper/gradle-wrapper.properties`
* `jdkVersion` in `gradle.properties`

## Flutter

Version from:

* `environment.flutter` in `pubspec.yaml`

## Examples

Here are a few examples of how `gg.cmd` can make your life easier:

### Execute gradle

```bash
./gg.cmd gradle build
```

### Replace gradlew and gradle JARs by `gg.cmd`

You can replace `gradlew` with a single `gg.cmd` and `gradle.properties` and can
delete these files:

* `gradle/wrapper/gradle-wrapper.jar`
* `gradle/wrapper/gradle-wrapper.properties`
* `gradlew`
* `gradlew.bat`

### Execute bld

In this example, [bld](https://rife2.com/bld) is used to run an app using `bld` for the build process:

```bash
./gg.cmd bld run
```

### Execute JBang

```bash
./gg.cmd jbang script.java
```

### Execute specific version of Node.js

```bash
./gg.cmd node@14
```

### Execute specific version of Gradle and the required version of JVM/JDK

```bash
./gg.cmd gradle@6:java@17 clean build
```

### Create a new node project

```bash
./gg.cmd npm init -y
```

### Create a new React project

```bash
./gg.cmd npx create-react-app my-app
cp gg.cmd my-app
cd my-app
./gg.cmd npm start
```

### Execute code hosted on GitHub

`gg.cmd` offers a GitHub executor.
It smartly checks the content and the available release files.

For instance, one can run [GitHub's CLI tool](https://cli.github.com/):

```bash
> sh ./gg.cmd gh/cli/cli --version
gh version 2.73.0 (2025-05-19)
https://github.com/cli/cli/releases/tag/v2.73.0
```

### Using Configuration and Aliases

Create a `gg.toml` configuration file:

```bash
./gg.cmd config init
```

Add some aliases and dependencies:

```toml
[dependencies]
node = "^18.0.0"
gradle = "~7.6.0"

[aliases]
dev = "node@18 server.js --dev"
build = "gradle clean build"
test = "npm test --coverage"
```

Now use the aliases:

```bash
./gg.cmd dev                # Runs: ./gg.cmd node@18 server.js --dev
./gg.cmd build --parallel   # Runs: ./gg.cmd gradle clean build --parallel
./gg.cmd test               # Runs: ./gg.cmd npm test --coverage
```

## Cache Management

`gg.cmd` supports both global and local cache modes:

### Global Cache (Default)

By default, tools are cached globally and shared across all projects:

- **Unix/Linux/macOS**: `$HOME/.cache/gg`
- **Windows**: `%UserProfile%\.cache\gg`

```bash
./gg.cmd node -v          # Uses global cache
./gg.cmd gradle build     # Uses global cache
```

### Local Cache

Use the `-l` flag to cache tools locally in the current project:

```bash
./gg.cmd -l node -v       # Uses .cache/gg in current directory
./gg.cmd -l gradle build  # Uses .cache/gg in current directory
```

### Custom Cache Location

Set the `GG_CACHE_DIR` environment variable to use a custom cache location:

```bash
export GG_CACHE_DIR="/path/to/custom/cache"
./gg.cmd node -v          # Uses /path/to/custom/cache
./gg.cmd -l node -v       # Still uses /path/to/custom/cache (ignores -l)
```

**Note**: When `GG_CACHE_DIR` is set, it takes precedence over both global and local cache modes.

## Contributing

We welcome contributions to `gg.cmd`. If you have an idea for a new feature or have found a bug, please open an issue on
the [GitHub repository](https://github.com/eirikb/gg).

## License

`gg.cmd` is licensed under the MIT License. See [LICENSE](LICENSE) for more information.
