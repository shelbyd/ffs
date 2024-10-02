# ffs

I am tired of all the shit to wade through when building and testing code. I hate making tools work together and getting a reliable, fast, incremental build system set up for every new project.

This is a Fast, Flexible, and Simple build system and task runner, ffs. Any similarity to internet slang is purely coincidental.

## Goals

```sh
git clone <my-project>
cd <my-project>
./ffs test # All tests run and pass
```

* Users don't need to install extra tools. Just clone and run.
* Integration with new languages, frameworks, or custom scripts is simple and works with existing plugins or other custom scripts.
* Builds produce byte-for-byte identical output if the underlying tools do.

## Model

ffs commands:

* run - Run all tasks matching the selector.

### Tasks and Builds

Tasks are defined with the `task` function in the FFS files. Tasks are run on the host machine and can run arbitrary commands. Tasks are recommended for things that primarily have side-effects like deploys, uploads, etc.

Most things should be `build`s. Builds only have access to their whitelisted input files and explicit environment variables. They are purely for producing other files. They can run on any remote build executor that you have configured.

### Targets

Every task and build can be referenced as a target.

* Target - A specific task or build. A task `foo` in /path/to/FFS would have the target string `//path/to/foo`.
* Selector - A matcher for multiple targets. `//path/to/...@foo` would match all targets that start with `//path/to/` and are tagged with `foo`.
* Output - A file produced by a target. `//path/to/foo:output` would reference the file produced by `//path/to/foo` named `output`. A "target" string can be used as an output and will refer to the output with the special name `default`.
* Relative Targets/Outputs - In the context of another target, you can refer to relative targets with `%/path/to`. So when defining `//some/target`, `%/another/target` would resolve to `//some/another/target`.
