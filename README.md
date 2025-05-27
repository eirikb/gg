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

## Support table

| Logo                                                                                                                          | Commands                         | Depends on | Set environment variables | Available tags                                                                     | Default tags |
|-------------------------------------------------------------------------------------------------------------------------------|----------------------------------|------------|---------------------------|------------------------------------------------------------------------------------|--------------|
| <img src="https://user-images.githubusercontent.com/241706/231715452-4e04052a-d13c-4bca-afa5-0bb19239b6f0.png" width="100px"> | **node**<br/>**npm**<br/>**npx** |            |                           | lts                                                                                |              |
| <img src="https://github.com/eirikb/gg/assets/241706/71b42988-bf62-49d3-b675-b2e526b3a8cc" width="100px">                     | **deno**                         |            |                           |                                                                                    |              |
| <img src="https://user-images.githubusercontent.com/241706/231713381-cc8436bb-ef6e-4aa6-ab5c-66ee0a868201.png" width="100px"> | **gradle**                       | java       |                           |                                                                                    |              |
| <img src="https://user-images.githubusercontent.com/241706/231999543-61a192f0-7931-495d-a845-fdd855e690e5.png" width="100px"> | **maven**<br/>**mvn**            | java       |                           |                                                                                    |              |
| <img src="https://www.jbang.dev/assets/images/logo.png" width="50%">                                                          | **jbang**                        | java       |                           |                                                                                    |              |
| <img src="https://user-images.githubusercontent.com/241706/231713130-ba667ff2-a129-47be-9d06-9e68e6815108.png" width="100px"> | **java**                         |            | JAVA_HOME                 | jdk<br/>jre<br/>lts<br/>sts<br/>mts<br/>ea<br/>ga<br/>headless<br/>headfull<br/>fx | +jdk<br/>+ga |
| <img src="https://github.com/eirikb/gg/assets/241706/e674f306-ce32-4718-b560-1b454f49e94c" width="100px">                     | **go**                           |            |                           | beta                                                                               |              |
| <img src="https://github.com/eirikb/gg/assets/241706/4d8be751-4680-4cc8-a939-f7ee6fac841f" width="100px">                     | **openapi**                      | java       |                           | beta                                                                               |              |

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
Currently, Java and Node-based projects are supported.

For instance, one can run [the Java build tool bld](https://rife2.com/bld) using `gh/rife2/bld`:

```bash
> sh gg.cmd gh/rife2/bld version
bld 2.2.1
```

## Contributing

We welcome contributions to gg.cmd. If you have an idea for a new feature or have found a bug, please open an issue on
the [GitHub repository](https://github.com/example/gg).

## License

`gg.cmd` is licensed under the MIT License. See [LICENSE](LICENSE) for more information.
