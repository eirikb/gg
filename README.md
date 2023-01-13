# gg.cmd - The Ultimate Executable Manager

Download:  
[gg.cmd](https://github.com/eirikb/m/releases/latest/download/gg.cmd)

Are you tired of managing multiple executables for different projects on your system? Introducing gg.cmd, the ultimate
cross-platform executable manager. With gg.cmd, you can easily switch between different executables with a single
command, saving you time and hassle.

## Features

- Simplify the management of other executables in your project
- Automatically detect and switch to the required executable version using project configuration files (such
  as `package.json` for Node.js projects)
- Support for chaining multiple executables (e.g. `gradle@6:java@17`)
- Cross-platform compatibility (Windows, macOS, and Linux)
- Easy executable switching with a single command
- Fast and lightweight

## Usage

Using gg.cmd is easy. Simply place the executable in the root of your project and run it with the `gg` command followed
by the desired executable and its required dependencies:

```bash
./gg.cmd <executable name>@<version>:<dependent executable name>@<version>
```

For example, to switch to a specific version of Gradle and the required version of Java, you can use the following
command:

```
./gg.cmd gradle@6:java@17
```

You can also specify multiple dependencies by separating them with a : symbol:

```
./gg.cmd gradle@6:java@17:maven@3.6.3
```

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
