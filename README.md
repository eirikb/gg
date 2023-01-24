# gg.cmd - The Ultimate Executable Manager

[![gg.cmd](https://github.com/eirikb/gg/actions/workflows/gg.yml/badge.svg)](https://github.com/eirikb/gg/actions/workflows/gg.yml)

Are you tired of managing multiple executables for different projects on your system? Introducing gg.cmd, the ultimate
cross-platform executable manager. With gg.cmd, you can easily switch between different executables with a single
command, saving you time and hassle.

Install into your repo:
> curl https://github.com/eirikb/m/releases/latest/download/gg.cmd > gg.cmd

## Features

- Simplify the management of other executables in your project
- Automatically detect and switch to the required executable version using project configuration files (such
  as `package.json` for Node.js projects)
- Support for chaining multiple executables (e.g. `gradle@6:java@17`)
- Cross-platform compatibility (Windows, macOS, and Linux)
- Easy executable switching with a single command
- Fast and lightweight

Install programs locally in a folder called `.cache`. Global install not supported.
Adds every dependency into `PATH` before executing.

## Usage

Using gg.cmd is easy. Simply place the executable in the root of your project and run it with the `gg.cmd` command
followed
by the desired executable and its required dependencies:

```bash
./gg.cmd [gg options] <executable name>@<version>:<dependent executable name>@<version> [executable arguments]
```

For example, to switch to a specific version of Gradle and the required version of Java, you can use the following
command:

```
./gg.cmd gradle@6:java@17
```

You can also specify multiple dependencies by separating them with a : symbol:

```
./gg.cmd gradle@6:java@17:node
```

#### Node

Supports `npm` and `npx` as well. Version specified refers to node version (not npm).

`engines` from `package.json` used to determine requried version.

For musl (e.g., alpine) unofficial builds are used ( https://unofficial-builds.nodejs.org/ ).

#### Gradle

`distributionUrl` from `gradle/wrapper/gradle-wrapper.properties` used find download url.

#### Java

Sets `JAVA_HOME`.

`jdkVersion` from `gradle/wrapper/gradle-wrapper.properties` used to determine required version.

## Examples

Here are a few examples of how gg.cmd can make your life easier:
<!--  Not yet
# Automatically switch to the required version of Node.js as specified in package.json
./gg
-->

# Switch to a specific version of Node

```
./gg.cmd node@14
```

# Switch to a specific version of Gradle and the required version of Java

```
./gg.cmd gradle@6:java@17
```

## Contributing

We welcome contributions to gg.cmd. If you have an idea for a new feature or have found a bug, please open an issue on
the [GitHub repository](https://github.com/example/gg).

## License

gg.cmd is licensed under the MIT License. See [LICENSE](LICENSE) for more information.
