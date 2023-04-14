# gg.cmd

[![gg.cmd](https://github.com/eirikb/gg/actions/workflows/gg.yml/badge.svg)](https://github.com/eirikb/gg/actions/workflows/gg.yml)

gg.cmd is a cross-platform and cross-architecture command-line interface (CLI) that acts as an executable wrapper for
various tools such as Gradle, JDK/JVM, Node.js, and Java. It requires minimal dependencies and is similar in
functionality to gradlew.

Install into root of your repo:
> wget gg.eirikb.no/gg.cmd

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

Using gg.cmd is easy. Simply place the executable in the root of your project and run it with the `gg.cmd` command
followed
by the desired executable and its required dependencies:

```bash
./gg.cmd [gg options] <executable name>@<version><+include_tags><-exclude_tags>:<dependent executable name>@<version><+include_tags><-exclude_tags> [executable arguments]
```

### Example

```bash
sh gg.cmd -v gradle@7:java+lts build
```

## Support table

| Logo                                                                                                          | Commands                         | Depends on | Version from                                                                                         | Download url from                                                                                                  | Set environment variables | Available tags                                                                     | Default tags |
|---------------------------------------------------------------------------------------------------------------|----------------------------------|------------|------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------|---------------------------|------------------------------------------------------------------------------------|--------------|
| ![image](https://user-images.githubusercontent.com/241706/231715452-4e04052a-d13c-4bca-afa5-0bb19239b6f0.png) | **node**<br/>**npm**<br/>**npx** |            | `engines` in `package.json`<br/> Contents of `.nvmrc`                                                |                                                                                                                    | lts                       |                                                                                    |
| ![image](https://user-images.githubusercontent.com/241706/231713381-cc8436bb-ef6e-4aa6-ab5c-66ee0a868201.png) | **gradle**                       | java       | `distributionUrl` in `gradle/wrapper/gradle-wrapper.properties`                                      | `distributionUrl` in<br/> `gradle/wrapper/gradle-wrapper.properties`<br/> `distributionUrl` in `gradle.properties` |                           |                                                                                    |
| ![image](https://user-images.githubusercontent.com/241706/231713130-ba667ff2-a129-47be-9d06-9e68e6815108.png) | **java**                         |            | `jdkVersion` in `gradle/wrapper/gradle-wrapper.properties` <br/> `jdkVersion` in `gradle.properties` |                                                                                                                    | JAVA_HOME                 | jdk<br/>jre<br/>lts<br/>sts<br/>mts<br/>ea<br/>ga<br/>headless<br/>headfull<br/>fx | +jdk         |
| ![image](https://user-images.githubusercontent.com/241706/231999543-61a192f0-7931-495d-a845-fdd855e690e5.png) | **maven**<br/>**mvn**            | java       |                                                                                                      |                                                                                                                    |                           |                                                                                    |              |

## On gradlew

With support for `distributionUrl` in `gradle.properties` you can replace gradlew with a single gg.cmd and can
delete these files:

* gradle/wrapper/gradle-wrapper.jar
* gradle/wrapper/gradle-wrapper.properties
* gradlew
* gradlew.bat

## On nvm

gg.cmd does not need to be installed beforehand, it is just a file in your repo that everyone can run.  
gg.cmd is a wrapper and will not tamper with system installations of Node.js.

```bash
nvm i 14
npm start
```

vs

```bash
./gg.cmd npm@14 start
```

## Examples

Here are a few examples of how gg.cmd can make your life easier:

### Execute gradle

```bash
./gg.cmd gradle build
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

## Contributing

We welcome contributions to gg.cmd. If you have an idea for a new feature or have found a bug, please open an issue on
the [GitHub repository](https://github.com/example/gg).

## License

gg.cmd is licensed under the MIT License. See [LICENSE](LICENSE) for more information.
