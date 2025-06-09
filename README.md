# `gg.cmd`

[![gg.cmd](https://github.com/eirikb/gg/actions/workflows/gg.yml/badge.svg)](https://github.com/eirikb/gg/actions/workflows/gg.yml)
[![Release](https://badgen.net/github/release/eirikb/gg)](https://github.com/eirikb/gg/releases/latest/download/gg.cmd)

[**[Changelog]**](https://github.com/eirikb/gg/releases)
[**[Download]**](https://github.com/eirikb/gg/releases/latest/download/gg.cmd)

![Image](https://github.com/user-attachments/assets/35d6bc4f-ea3b-4673-a874-45703c4de1d8)

![Image](https://github.com/user-attachments/assets/93876050-9c28-4389-a77c-5a88f5af2811)

`gg.cmd` is a cross-platform and cross-architecture command-line interface (CLI) that acts as an executable wrapper for
various tools such as Gradle, JDK/JVM, Node.js, and Java. It requires minimal dependencies and is similar in
functionality to gradlew.

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

- Simplify the management of other executables in your project
- Automatically detect and execute the required executable version using project configuration files (such
  as `package.json` for Node.js projects)
- Support for chaining multiple executables (e.g. `gradle@6:java@17`)
- Cross-platform compatibility (Windows, macOS, and Linux)
- Cross-architecture compatibility (x86_64 and ARM)
- Fast and lightweight

Installs tool locally in a folder called `.cache`. Global install not supported.
Adds every dependency into `PATH` before executing.

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
    -v              Info output
    -vv             Debug output
    -vvv            Trace output
    -w              Even more output
    -V, --version   Print version
    --os <OS>       Override target OS (windows, linux, mac)
    --arch <ARCH>   Override target architecture (x86_64, arm64, armv7)

Built in commands:
    update          Update gg.cmd
    help            Print help
    check           Check for updates
    check-update    Check for updates and update if available
    clean-cache     Clean cache

Version syntax:
    @X              Any X.y.z version (e.g. node@14 for any Node.js 14.x.y)
    @X.Y            Any X.Y.z patch version (e.g. node@14.17 for any Node.js 14.17.z)
    @X.Y.Z          Exactly X.Y.Z version (e.g. node@14.17.0 for exactly Node.js 14.17.0)
    @^X.Y.Z         X.Y.Z or any compatible newer version (caret prefix)
    @~X.Y.Z         X.Y.Z or any newer patch version (tilde prefix)
    @=X.Y.Z         Exactly X.Y.Z version (equals prefix, same as X.Y.Z without prefix)

Supported systems:
    node (npm, npx will also work, version refers to node version)
    gradle
    java
    jbang
    maven (mvn)
    bld
    openapi
    rat (ra)
    deno
    go
    caddy
    just
    fortio
    run (any arbitrary command)
    gh/<owner>/<repo> (GitHub releases)

Available tags by system:
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
It smartly checks if the content and the available release files.

For instance, one can run [GitHub's CLI tool](https://cli.github.com/):

```bash
> sh ./gg.cmd gh/cli/cli --verison
gh version 2.73.0 (2025-05-19)
https://github.com/cli/cli/releases/tag/v2.73.0
```

## Contributing

We welcome contributions to `gg.cmd`. If you have an idea for a new feature or have found a bug, please open an issue on
the [GitHub repository](https://github.com/eirikb/gg).

## License

`gg.cmd` is licensed under the MIT License. See [LICENSE](LICENSE) for more information.
